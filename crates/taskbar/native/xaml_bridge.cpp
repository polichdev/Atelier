#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <unknwn.h>
#include <ocidl.h>
#include <xamlom.h>
#include <shobjidl.h>
#include <shellapi.h>
#include <filesystem>
#include <cwctype>
#undef GetCurrentTime
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.UI.h>
#include <winrt/Windows.UI.Xaml.h>
#include <winrt/Windows.UI.Xaml.Media.h>
#include <winrt/Windows.UI.Xaml.Shapes.h>
#include <winrt/Windows.UI.Xaml.Controls.h>
#include <winrt/Windows.UI.Xaml.Input.h>
#include <winrt/Windows.UI.Xaml.Data.h>
#include <winrt/Windows.UI.Xaml.Media.Imaging.h>
#include <winrt/Windows.UI.Xaml.Automation.Peers.h>
#pragma comment(lib,"shell32.lib")
#pragma comment(lib,"gdi32.lib")
#pragma comment(lib,"uuid.lib")
#pragma comment(lib,"advapi32.lib")
#include <winrt/Windows.Foundation.Collections.h>
#include <atomic>
#include <algorithm>
#include <map>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

namespace xaml = winrt::Windows::UI::Xaml;
namespace media = xaml::Media;
namespace shapes = xaml::Shapes;
using winrt::Windows::UI::Color;
thread_local std::wstring highResInfo;

std::wstring lower(std::wstring value){for(auto& c:value)c=static_cast<wchar_t>(std::towlower(c));return value;}
xaml::FrameworkElement appAncestor(xaml::DependencyObject e){for(int depth=0;e&&depth<32;++depth){if(auto f=e.try_as<xaml::FrameworkElement>()){if(f.Name()==L"TaskListButton"||winrt::get_class_name(f)==L"Taskbar.TaskListButton")return f;}e=media::VisualTreeHelper::GetParent(e);}return nullptr;}
struct __declspec(uuid("905a0fef-bc53-11df-8c49-001e4fc686da")) PixelBufferAccess : IUnknown {virtual HRESULT STDMETHODCALLTYPE Buffer(BYTE** value)=0;};
media::ImageSource shellCachedIcon(std::wstring const& id){
    if(id.rfind(L"appid:",0)!=0)return nullptr;
    auto app=id.substr(6);while(!app.empty()&&iswspace(app.front()))app.erase(app.begin());
    winrt::com_ptr<IShellItemImageFactory> factory;
    if(FAILED(SHCreateItemFromParsingName((L"shell:AppsFolder\\"+app).c_str(),nullptr,IID_PPV_ARGS(factory.put()))))return nullptr;
    HBITMAP bitmap=nullptr;
    if(FAILED(factory->GetImage({128,128},static_cast<SIIGBF>(SIIGBF_ICONONLY|SIIGBF_BIGGERSIZEOK|SIIGBF_INCACHEONLY),&bitmap))||!bitmap)return nullptr;
    BITMAP dimensions{};GetObjectW(bitmap,sizeof(dimensions),&dimensions);
    BITMAPINFO info{};info.bmiHeader.biSize=sizeof(BITMAPINFOHEADER);info.bmiHeader.biWidth=dimensions.bmWidth;info.bmiHeader.biHeight=-dimensions.bmHeight;info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=32;
    media::Imaging::WriteableBitmap source(dimensions.bmWidth,dimensions.bmHeight);BYTE* bytes=nullptr;winrt::check_hresult(source.PixelBuffer().as<PixelBufferAccess>()->Buffer(&bytes));
    HDC dc=CreateCompatibleDC(nullptr);const auto lines=GetDIBits(dc,bitmap,0,dimensions.bmHeight,bytes,&info,DIB_RGB_COLORS);DeleteDC(dc);DeleteObject(bitmap);
    if(!lines)return nullptr;source.Invalidate();return source;
}
media::ImageSource highResolutionIcon(xaml::FrameworkElement const& element){
    try{
    static thread_local std::map<std::wstring,media::ImageSource> cache;
    auto button=appAncestor(element);if(!button)return nullptr;
    auto peer=xaml::Automation::Peers::FrameworkElementAutomationPeer::CreatePeerForElement(button);if(!peer)return nullptr;
    const auto name=lower(peer.GetName().c_str());const auto id=lower(peer.GetAutomationId().c_str());const auto key=id.empty()?name:id;
    highResInfo=name+L" | "+id;
    if(key.empty())return nullptr;if(auto found=cache.find(key);found!=cache.end()){highResInfo+=found->second?L" | cached high resolution":L" | no matching icon";return found->second;}
    std::wstring iconPath;int iconIndex=0;
    if(id.rfind(L"appid:",0)==0){auto candidate=id.substr(6);while(!candidate.empty()&&iswspace(candidate.front()))candidate.erase(candidate.begin());std::error_code error;if(std::filesystem::is_regular_file(candidate,error))iconPath=candidate;}
    if(auto roaming=_wgetenv(L"APPDATA")){
        const auto directory=std::filesystem::path(roaming)/L"Microsoft/Internet Explorer/Quick Launch/User Pinned/TaskBar";
        std::error_code error;size_t best=0;
        for(auto const& entry:std::filesystem::directory_iterator(directory,error)){
            if(entry.path().extension()!=L".lnk")continue;const auto stem=lower(entry.path().stem().wstring());
            if(stem.empty()||stem.size()<best||(name.find(stem)==std::wstring::npos&&id.find(stem)==std::wstring::npos))continue;
            winrt::com_ptr<IShellLinkW> link;if(FAILED(CoCreateInstance(CLSID_ShellLink,nullptr,CLSCTX_INPROC_SERVER,IID_PPV_ARGS(link.put()))))continue;
            if(FAILED(link.as<IPersistFile>()->Load(entry.path().c_str(),STGM_READ)))continue;
            wchar_t path[32768]{};int index=0;link->GetIconLocation(path,32768,&index);
            if(!*path){link->GetPath(path,32768,nullptr,0);index=0;}
            if(*path){iconPath=path;iconIndex=index;best=stem.size();}
        }
    }
    if(iconPath.empty()){
        struct Search{std::wstring name,path;size_t score=0;} search{name};
        EnumWindows([](HWND h,LPARAM data)->BOOL{
            auto& s=*reinterpret_cast<Search*>(data);if(!IsWindowVisible(h))return TRUE;
            wchar_t title[1024]{};if(!GetWindowTextW(h,title,1024))return TRUE;auto text=lower(title);
            if(s.name.find(text)==std::wstring::npos&&text.find(s.name)==std::wstring::npos)return TRUE;
            if(text.size()<s.score)return TRUE;DWORD pid=0;GetWindowThreadProcessId(h,&pid);HANDLE p=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,pid);if(!p)return TRUE;
            wchar_t path[32768]{};DWORD size=32768;if(QueryFullProcessImageNameW(p,0,path,&size)){s.path.assign(path,size);s.score=text.size();}CloseHandle(p);return TRUE;
        },reinterpret_cast<LPARAM>(&search));iconPath=search.path;
    }
    if(iconPath.empty()){if(auto source=shellCachedIcon(id)){highResInfo+=L" | Windows app icon cache";cache.emplace(key,source);return source;}cache.emplace(key,nullptr);return nullptr;}
    highResInfo+=L" | "+iconPath;
    wchar_t expanded[32768]{};if(ExpandEnvironmentStringsW(iconPath.c_str(),expanded,32768))iconPath=expanded;
    using Extract=UINT(WINAPI*)(LPCWSTR,int,int,int,HICON*,UINT*,UINT,UINT);
    auto extract=reinterpret_cast<Extract>(GetProcAddress(GetModuleHandleW(L"user32.dll"),"PrivateExtractIconsW"));
    HICON icon=nullptr;if(!extract||!extract(iconPath.c_str(),iconIndex,128,128,&icon,nullptr,1,0)||!icon){cache.emplace(key,nullptr);return nullptr;}
    BITMAPINFO info{};info.bmiHeader.biSize=sizeof(BITMAPINFOHEADER);info.bmiHeader.biWidth=128;info.bmiHeader.biHeight=-128;info.bmiHeader.biPlanes=1;info.bmiHeader.biBitCount=32;
    HDC dc=CreateCompatibleDC(nullptr);void* pixels=nullptr;HBITMAP bitmap=CreateDIBSection(dc,&info,DIB_RGB_COLORS,&pixels,nullptr,0);
    if(!dc||!bitmap||!pixels){if(bitmap)DeleteObject(bitmap);if(dc)DeleteDC(dc);DestroyIcon(icon);return nullptr;}
    auto previous=SelectObject(dc,bitmap);memset(pixels,0,128*128*4);DrawIconEx(dc,0,0,icon,128,128,0,nullptr,DI_NORMAL);
    media::Imaging::WriteableBitmap source(128,128);BYTE* destination=nullptr;winrt::check_hresult(source.PixelBuffer().as<PixelBufferAccess>()->Buffer(&destination));memcpy(destination,pixels,128*128*4);source.Invalidate();
    SelectObject(dc,previous);DeleteObject(bitmap);DeleteDC(dc);DestroyIcon(icon);cache.emplace(key,source);return source;
    }catch(...){return nullptr;}
}

namespace {
#ifndef ATELIER_BRIDGE_REVISION
#define ATELIER_BRIDGE_REVISION 0ULL
#endif
constexpr GUID TapClass{static_cast<DWORD>(0x3a52de99^(ATELIER_BRIDGE_REVISION&0xffffffff)),static_cast<WORD>(0x68a5^(ATELIER_BRIDGE_REVISION>>32)),static_cast<WORD>(0x49cd^(ATELIER_BRIDGE_REVISION>>48)),{0x9b,0x51,0xb0,0x44,0xd5,0x9f,0x65,0x04}};
constexpr DWORD Magic = 0x41544232;
constexpr size_t SlotCount = 16;
enum Material : DWORD { System = 0, Clear = 1, Blur = 2, Acrylic = 3 };
struct Slot {
    volatile LONG thread;
    volatile LONG targets;
    volatile LONG acknowledged;
    volatile LONG error;
    volatile LONG buttons;
    volatile LONG icons;
    volatile LONG iconPixels;
    volatile LONG buttonPixels;
    volatile LONG iconWidth;
    volatile LONG iconMinimum;
    volatile LONG iconMaximum;
    volatile LONG sourcePixels;
    volatile LONG repairAcknowledged;
    volatile LONG highResolutionIcons;
    volatile LONG upscaledIcons;
    wchar_t description[1024];
};
struct Shared {
    DWORD magic;
    volatile LONG sequence;
    DWORD owner;
    DWORD material;
    DWORD color;
    volatile LONG attached;
    volatile LONG attachError;
    DWORD appearanceOwner;
    DWORD iconSize;
    DWORD buttonWidth;
    LONG buttonRadius;
    DWORD inactiveOpacity;
    DWORD indicator;
    DWORD hoverScale;
    LONG repairEpoch;
    Slot slots[SlotCount];
};
struct Mapping {
    HANDLE handle = nullptr;
    Shared* data = nullptr;
    ~Mapping() {
        if (data) UnmapViewOfFile(data);
        if (handle) CloseHandle(handle);
    }
    void open(DWORD pid, bool create) {
        const auto name = L"Local\\Atelier.Taskbar.Brush.V2." + std::to_wstring(ATELIER_BRIDGE_REVISION)+L"."+std::to_wstring(pid);
        handle = create ? CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE,
                                             0, sizeof(Shared), name.c_str())
                        : OpenFileMappingW(FILE_MAP_ALL_ACCESS, FALSE, name.c_str());
        winrt::check_bool(handle != nullptr);
        data = static_cast<Shared*>(MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, sizeof(Shared)));
        winrt::check_bool(data != nullptr);
        if (create && data->magic == 0) data->magic = Magic;
        winrt::check_bool(data->magic == Magic);
    }
};
struct Desired { LONG sequence; DWORD owner; DWORD material; DWORD color; DWORD appearanceOwner; DWORD iconSize; DWORD buttonWidth; LONG buttonRadius; DWORD inactiveOpacity; DWORD indicator; DWORD hoverScale; };
bool snapshot(Shared* shared, Desired& value) noexcept {
    const LONG first = InterlockedCompareExchange(&shared->sequence, 0, 0);
    if (first & 1) return false;
    value = {first, shared->owner, shared->material, shared->color, shared->appearanceOwner, shared->iconSize, shared->buttonWidth, shared->buttonRadius, shared->inactiveOpacity, shared->indicator, shared->hoverScale};
    MemoryBarrier();
    return first == InterlockedCompareExchange(&shared->sequence, 0, 0);
}
struct Target {
    InstanceHandle handle;
    winrt::weak_ref<shapes::Rectangle> rectangle;
    winrt::Windows::Foundation::IInspectable originalLocal{nullptr};
    winrt::Windows::Foundation::IInspectable originalStroke{nullptr};
    double originalStrokeThickness = 1.0;
    xaml::Visibility originalVisibility = xaml::Visibility::Visible;
    double originalOpacity = 1.0;
    media::Brush applied{nullptr};
    bool border;
    bool modified = false;
    LONG sequence = -1;
};
enum ElementKind { AppButton, AppIcon, AppIndicator, AppBackground };
struct ElementTarget {
    InstanceHandle handle;
    winrt::weak_ref<xaml::FrameworkElement> element;
    ElementKind kind;
    std::vector<std::pair<xaml::DependencyProperty,winrt::Windows::Foundation::IInspectable>> originals;
    bool pointerOver = false;
    winrt::event_token entered{}, exited{};
    media::ScaleTransform scale{nullptr};
    double naturalIconSize=0;
    LONG repairEpoch=0;
    std::wstring description;
    void remember(xaml::FrameworkElement const& e, xaml::DependencyProperty const& dp) {
        if (std::none_of(originals.begin(),originals.end(),[&](auto const& item){return item.first==dp;})) {
            auto value=e.ReadLocalValue(dp);
            try{if(auto binding=e.GetBindingExpression(dp))value=binding.ParentBinding();}catch(...){}
            originals.emplace_back(dp,value);
        }
    }
    void putBack(xaml::FrameworkElement const& e,xaml::DependencyProperty const& dp,winrt::Windows::Foundation::IInspectable const& value){
        if(auto binding=value.try_as<xaml::Data::Binding>())e.SetBinding(dp,binding);
        else if(value==xaml::DependencyProperty::UnsetValue())e.ClearValue(dp);else e.SetValue(dp,value);
    }
    void restoreProperty(xaml::FrameworkElement const& e,xaml::DependencyProperty const& dp) {
        auto it=std::find_if(originals.begin(),originals.end(),[&](auto const& item){return item.first==dp;});
        if(it==originals.end())return;
        try{putBack(e,dp,it->second);}catch(...){e.ClearValue(dp);}
        originals.erase(it);
    }
    void restore(xaml::FrameworkElement const& e) {
        for(auto const& item:originals){try{putBack(e,item.first,item.second);}catch(...){try{e.ClearValue(item.first);}catch(...){}}} originals.clear();
        scale = nullptr;
        naturalIconSize = 0;
        pointerOver = false;
    }
    ~ElementTarget(){try{if(auto e=element.get()){if(entered.value)e.PointerEntered(entered);if(exited.value)e.PointerExited(exited);}}catch(...){}}
};
bool isAppButton(xaml::FrameworkElement const& e) {
    const std::wstring type=winrt::get_class_name(e).c_str();
    return e.Name()==L"TaskListButton" || type==L"Taskbar.TaskListButton";
}
bool hasAppButtonParent(xaml::DependencyObject e) {
    for(int depth=0;e&&depth<32;++depth){if(auto f=e.try_as<xaml::FrameworkElement>()){if(isAppButton(f))return true;}e=media::VisualTreeHelper::GetParent(e);}return false;
}
bool activeState(xaml::FrameworkElement const& e,int depth=0) {
    for(auto const& group:xaml::VisualStateManager::GetVisualStateGroups(e)) {
        if(auto state=group.CurrentState()){const std::wstring name=state.Name().c_str();if(name.find(L"Active")!=std::wstring::npos)return true;}
    }
    if(depth>=4)return false;
    for(int i=0,n=media::VisualTreeHelper::GetChildrenCount(e);i<n;++i){if(auto child=media::VisualTreeHelper::GetChild(e,i).try_as<xaml::FrameworkElement>()){if(activeState(child,depth+1))return true;}}
    return false;
}
struct UiContext {
    std::shared_ptr<Mapping> mapping;
    Slot* slot = nullptr;
    std::vector<Target> targets;
    std::vector<std::shared_ptr<ElementTarget>> elements;
    DWORD appearanceOwner=0;
    HANDLE appearanceHandle=nullptr;
    DWORD owner = 0;
    HANDLE ownerHandle = nullptr;
    UINT_PTR timer = 0;
    bool resetPerformed = false;
    bool brushModified = false;

    explicit UiContext(std::shared_ptr<Mapping> value) : mapping(std::move(value)) {
        for (auto& candidate : mapping->data->slots) {
            if (InterlockedCompareExchange(&candidate.thread, GetCurrentThreadId(), 0) == 0) {
                slot = &candidate;
                break;
            }
        }
        if (!slot) winrt::throw_hresult(HRESULT_FROM_WIN32(ERROR_TOO_MANY_SESS));
    }
    void restore(Target& target, shapes::Rectangle const& rectangle) {
        if (!target.modified) return;
        rectangle.Visibility(target.originalVisibility);
        rectangle.Opacity(target.originalOpacity);
        if (target.originalLocal == xaml::DependencyProperty::UnsetValue()) {
            rectangle.ClearValue(shapes::Shape::FillProperty());
        } else {
            rectangle.SetValue(shapes::Shape::FillProperty(), target.originalLocal);
        }
        if (target.border) {
            if (target.originalStroke == xaml::DependencyProperty::UnsetValue()) {
                rectangle.ClearValue(shapes::Shape::StrokeProperty());
            } else {
                rectangle.SetValue(shapes::Shape::StrokeProperty(), target.originalStroke);
            }
            rectangle.StrokeThickness(target.originalStrokeThickness);
        }
        target.modified = false;
        target.applied = nullptr;
        target.sequence = -1;
    }
    bool ownerAlive(Desired const& desired) noexcept {
        if (owner != desired.owner) {
            if (ownerHandle) CloseHandle(ownerHandle);
            owner = desired.owner;
            ownerHandle = owner ? OpenProcess(SYNCHRONIZE, FALSE, owner) : nullptr;
        }
        return ownerHandle && WaitForSingleObject(ownerHandle, 0) == WAIT_TIMEOUT;
    }
    bool appearanceAlive(Desired const& desired) noexcept {
        if(appearanceOwner!=desired.appearanceOwner){if(appearanceHandle)CloseHandle(appearanceHandle);appearanceOwner=desired.appearanceOwner;appearanceHandle=appearanceOwner?OpenProcess(SYNCHRONIZE,FALSE,appearanceOwner):nullptr;}
        return appearanceHandle&&WaitForSingleObject(appearanceHandle,0)==WAIT_TIMEOUT;
    }
    void updateAppearance(Desired const& desired) {
        const bool reset=!appearanceAlive(desired);
        LONG buttons=0,icons=0;
        InterlockedExchange(&slot->highResolutionIcons,0);InterlockedExchange(&slot->upscaledIcons,0);
        InterlockedExchange(&slot->buttonPixels,0);InterlockedExchange(&slot->iconPixels,0);
        for(auto it=elements.begin();it!=elements.end();){
            auto target=*it;auto e=target->element.get();if(!e){it=elements.erase(it);continue;}
            const LONG repair=mapping->data->repairEpoch;
            if(repair&&target->repairEpoch!=repair){
                target->restore(e);
                e.InvalidateMeasure();e.InvalidateArrange();
                target->naturalIconSize=0;target->repairEpoch=repair;
            }
            if(target->kind==AppButton){++buttons;if(e.ActualWidth()>0)InterlockedExchange(&slot->buttonPixels,static_cast<LONG>(e.ActualWidth()*100+.5));}if(target->kind==AppIcon){++icons;if(e.ActualWidth()>0){double width=e.ActualWidth();const double factor=target->scale&&e.RenderTransform()==target->scale?target->scale.ScaleX():1.0;InterlockedExchange(&slot->iconWidth,static_cast<LONG>(e.Width()*100+.5));InterlockedExchange(&slot->iconMinimum,static_cast<LONG>(e.MinWidth()*100+.5));InterlockedExchange(&slot->iconMaximum,static_cast<LONG>(e.MaxWidth()*100+.5));if(auto image=e.try_as<xaml::Controls::Image>()){if(auto source=image.Source().try_as<media::Imaging::BitmapSource>()){InterlockedExchange(&slot->sourcePixels,source.PixelWidth());if(source.PixelWidth()>=64)InterlockedIncrement(&slot->highResolutionIcons);if(image.Stretch()==media::Stretch::None)width=(std::min)(width,source.PixelWidth()/(e.XamlRoot()?e.XamlRoot().RasterizationScale():1.0));}}if(auto image=e.try_as<xaml::Controls::Image>()){if(auto source=image.Source().try_as<media::Imaging::BitmapSource>()){if(width*factor*(e.XamlRoot()?e.XamlRoot().RasterizationScale():1.0)>source.PixelWidth()+.5)InterlockedIncrement(&slot->upscaledIcons);}}InterlockedExchange(&slot->iconPixels,static_cast<LONG>(width*factor*100+.5));}}
            if(reset){target->restore(e);++it;continue;}
            if(target->kind==AppIcon){
                DWORD iconSize=desired.iconSize;
                auto button=appAncestor(e);bool hovered=false;
                for(auto const& item:elements){if(item->kind==AppButton&&item->element.get()==button){hovered=item->pointerOver;break;}}
                if(!target->naturalIconSize&&e.ActualWidth()>0)target->naturalIconSize=e.ActualWidth();
                if(hovered&&desired.hoverScale>100)iconSize=static_cast<DWORD>((iconSize?iconSize:target->naturalIconSize)*desired.hoverScale/100.0+.5);
                auto image=e.try_as<xaml::Controls::Image>();
                if(iconSize&&image){
                    if(auto high=highResolutionIcon(e)){target->remember(e,xaml::Controls::Image::SourceProperty());if(image.Source()!=high)image.Source(high);}
                    target->remember(e,xaml::Controls::Image::StretchProperty());image.Stretch(media::Stretch::Uniform);
                    target->description=highResInfo;if(e.ActualWidth()>0)wcsncpy_s(slot->description,target->description.c_str(),_TRUNCATE);
                }else{target->restoreProperty(e,xaml::Controls::Image::SourceProperty());target->restoreProperty(e,xaml::Controls::Image::StretchProperty());}
                for(auto dp:{xaml::FrameworkElement::WidthProperty(),xaml::FrameworkElement::HeightProperty(),xaml::FrameworkElement::MinWidthProperty(),xaml::FrameworkElement::MaxWidthProperty(),xaml::FrameworkElement::MinHeightProperty(),xaml::FrameworkElement::MaxHeightProperty()}){
                    if(iconSize){target->remember(e,dp);e.SetValue(dp,winrt::box_value(double(iconSize)));}else target->restoreProperty(e,dp);
                }
                auto transform=xaml::UIElement::RenderTransformProperty();auto origin=xaml::UIElement::RenderTransformOriginProperty();auto cache=xaml::UIElement::CacheModeProperty();
                if(iconSize&&e.ActualWidth()>0&&image){
                    const auto bitmap=image.Source().try_as<media::Imaging::BitmapSource>();
                    const double dpi=e.XamlRoot()?e.XamlRoot().RasterizationScale():1.0;
                    const double sharpSize=(bitmap&&bitmap.PixelWidth()>0)?(std::min)(double(iconSize),bitmap.PixelWidth()/dpi):double(iconSize);
                    target->remember(e,transform);target->remember(e,origin);target->remember(e,cache);e.CacheMode(nullptr);
                    if(!target->scale)target->scale=media::ScaleTransform();
                    const double factor=sharpSize/e.ActualWidth();
                    if(target->scale.ScaleX()!=factor||target->scale.ScaleY()!=factor){
                        target->scale.ScaleX(factor);target->scale.ScaleY(factor);
                    }
                    e.RenderTransformOrigin({.5f,.5f});
                    if(e.RenderTransform()!=target->scale)e.RenderTransform(target->scale);
                }else{
                    target->scale=nullptr;
                    target->restoreProperty(e,transform);target->restoreProperty(e,origin);target->restoreProperty(e,cache);
                }
            } else if(target->kind==AppButton){
                for(auto dp:{xaml::FrameworkElement::WidthProperty(),xaml::FrameworkElement::MinWidthProperty(),xaml::FrameworkElement::MaxWidthProperty()}){
                    if(desired.buttonWidth){target->remember(e,dp);e.SetValue(dp,winrt::box_value(double(desired.buttonWidth)));}else target->restoreProperty(e,dp);
                }
                auto opacity=xaml::UIElement::OpacityProperty();
                if(desired.inactiveOpacity<100){target->remember(e,opacity);const double value=activeState(e)?1.0:desired.inactiveOpacity/100.0;if(e.Opacity()!=value)e.Opacity(value);}else target->restoreProperty(e,opacity);
                auto transform=xaml::UIElement::RenderTransformProperty();auto origin=xaml::UIElement::RenderTransformOriginProperty();
                target->restoreProperty(e,transform);target->restoreProperty(e,origin);
            } else if(target->kind==AppBackground){
                auto border=e.try_as<xaml::Controls::Border>();auto dp=xaml::Controls::Border::CornerRadiusProperty();
                if(border&&desired.buttonRadius>=0){target->remember(e,dp);double r=desired.buttonRadius;border.CornerRadius({r,r,r,r});}else target->restoreProperty(e,dp);
            } else if(target->kind==AppIndicator){
                auto dp=xaml::UIElement::VisibilityProperty();
                if(desired.indicator==3){target->remember(e,dp);e.Visibility(xaml::Visibility::Collapsed);}else target->restoreProperty(e,dp);
                auto rectangle=e.try_as<shapes::Rectangle>();
                for(auto property:{xaml::FrameworkElement::WidthProperty(),xaml::FrameworkElement::HeightProperty(),shapes::Rectangle::RadiusXProperty(),shapes::Rectangle::RadiusYProperty()}){
                    if(rectangle&&(desired.indicator==1||desired.indicator==2)){
                        target->remember(e,property);double value=(property==xaml::FrameworkElement::WidthProperty())?(desired.indicator==1?4.:16.):(property==xaml::FrameworkElement::HeightProperty())?(desired.indicator==1?4.:3.):(desired.indicator==1?2.:1.5);e.SetValue(property,winrt::box_value(value));
                    }else target->restoreProperty(e,property);
                }
            }
            ++it;
        }
        InterlockedExchange(&slot->buttons,buttons);InterlockedExchange(&slot->icons,icons);
        InterlockedExchange(&slot->repairAcknowledged,mapping->data->repairEpoch);
    }
    void update() noexcept {
        try {
            Desired desired{};
            if (!snapshot(mapping->data, desired)) return;
            updateAppearance(desired);
            const bool reset = desired.material == System || !ownerAlive(desired);
            if (reset && !resetPerformed && brushModified) {
                HWND tray = FindWindowW(L"Shell_TrayWnd", nullptr);
                if (tray) {
                    SetWindowRgn(tray, nullptr, TRUE);
                    HWND bridge = FindWindowExW(tray, nullptr, L"Windows.UI.Composition.DesktopWindowContentBridge", nullptr);
                    if (bridge) {
                        RECT r{};
                        if (GetWindowRect(tray, &r)) {
                            SetWindowPos(bridge, nullptr, 0, 0, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
                            SetWindowRgn(bridge, nullptr, TRUE);
                        }
                    }
                    RedrawWindow(tray, nullptr, nullptr, RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_FRAME | RDW_UPDATENOW);
                }
                resetPerformed = true;
                brushModified = false;
            } else if (!reset) {
                resetPerformed = false;
                brushModified = true;
            }
            LONG error = S_OK;
            LONG count = 0;
            for (auto it = targets.begin(); it != targets.end();) {
                auto rectangle = it->rectangle.get();
                if (!rectangle) { it = targets.erase(it); continue; }
                if (!it->border) ++count;
                try {
                    if (reset) {
                        restore(*it, rectangle);
                    } else {
                        if (it->sequence != desired.sequence || !it->applied) {
                            it->applied = media::SolidColorBrush(Color{0, 0, 0, 0});
                            it->sequence = desired.sequence;
                        }
                        if (rectangle.Visibility() != xaml::Visibility::Collapsed) {
                            rectangle.Visibility(xaml::Visibility::Collapsed);
                        }
                        if (rectangle.Opacity() != 0.0) {
                            rectangle.Opacity(0.0);
                        }
                        if (rectangle.Fill() != it->applied) rectangle.Fill(it->applied);
                        if (it->border) {
                            if (rectangle.Stroke() != it->applied) rectangle.Stroke(it->applied);
                            if (rectangle.StrokeThickness() != 0.0) rectangle.StrokeThickness(0.0);
                        }
                        it->modified = true;
                    }
                } catch (...) { error = winrt::to_hresult(); }
                ++it;
            }
            InterlockedExchange(&slot->targets, count);
            InterlockedExchange(&slot->error, error);
            InterlockedExchange(&slot->acknowledged, desired.sequence);
        } catch (...) {
            InterlockedExchange(&slot->error, winrt::to_hresult());
        }
    }
};
thread_local UiContext* ui = nullptr;
void CALLBACK tick(HWND, UINT, UINT_PTR, DWORD) noexcept { if (ui) ui->update(); }

bool isTaskbarElement(xaml::DependencyObject object) {
    for (int depth = 0; object && depth < 64; ++depth) {
        if (auto element = object.try_as<xaml::FrameworkElement>()) {
            const std::wstring name = element.Name().c_str();
            const std::wstring className = winrt::get_class_name(element).c_str();
            if (name == L"TaskbarFrame" || name == L"TaskbarView" || className.find(L"Taskbar") != std::wstring::npos) return true;
        }
        object = media::VisualTreeHelper::GetParent(object);
    }
    return false;
}

class Tap final : public IObjectWithSite, public IVisualTreeServiceCallback2 {
    std::atomic<ULONG> references{1};
    winrt::com_ptr<IXamlDiagnostics> diagnostics;
    std::shared_ptr<Mapping> mapping;
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** out) noexcept override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (iid == __uuidof(IUnknown) || iid == __uuidof(IObjectWithSite)) *out = static_cast<IObjectWithSite*>(this);
        else if (iid == __uuidof(IVisualTreeServiceCallback) || iid == __uuidof(IVisualTreeServiceCallback2)) *out = static_cast<IVisualTreeServiceCallback2*>(this);
        else return E_NOINTERFACE;
        AddRef(); return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() noexcept override { return ++references; }
    ULONG STDMETHODCALLTYPE Release() noexcept override { const auto n = --references; if (!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE GetSite(REFIID iid, void** out) noexcept override {
        if (!out) return E_POINTER;
        *out = nullptr;
        return diagnostics ? diagnostics->QueryInterface(iid, out) : E_FAIL;
    }
    HRESULT STDMETHODCALLTYPE SetSite(IUnknown* site) noexcept override {
        try {
            if (!site) return S_OK;
            if (diagnostics) return E_UNEXPECTED;
            winrt::check_hresult(site->QueryInterface(__uuidof(IXamlDiagnostics), diagnostics.put_void()));
            mapping = std::make_shared<Mapping>();
            mapping->open(GetCurrentProcessId(), false);
            AddRef();
            try {
                std::thread([this]() noexcept {
                    HRESULT result = E_FAIL;
                    try {
                        auto service = diagnostics.as<IVisualTreeService>();
                        result = service->AdviseVisualTreeChange(this);
                    } catch (...) { result = winrt::to_hresult(); }
                    InterlockedExchange(&mapping->data->attachError, result);
                    InterlockedExchange(&mapping->data->attached, SUCCEEDED(result) ? 1 : -1);
                    Release();
                }).detach();
            } catch (...) { Release(); throw; }
            return S_OK;
        } catch (...) { return winrt::to_hresult(); }
    }
    HRESULT STDMETHODCALLTYPE OnVisualTreeChange(ParentChildRelation, VisualElement element, VisualMutationType mutation) noexcept override {
        struct Strings { VisualElement value; ~Strings() {
            SysFreeString(value.SrcInfo.FileName); SysFreeString(value.SrcInfo.Hash);
            SysFreeString(value.Type); SysFreeString(value.Name);
        }} strings{mutation == Add ? element : VisualElement{}};
        try {
            if (mutation == Remove) {
                if (ui) {
                    ui->targets.erase(std::remove_if(ui->targets.begin(), ui->targets.end(),
                        [&](Target const& value) { return value.handle == element.Handle; }), ui->targets.end());
                    ui->elements.erase(std::remove_if(ui->elements.begin(),ui->elements.end(),[&](auto const& item){return item->handle==element.Handle;}),ui->elements.end());
                    ui->update();
                }
                return S_OK;
            }
            if (!element.Name) return S_OK;
            const std::wstring name(element.Name, SysStringLen(element.Name));
            const bool brushTarget=name==L"BackgroundFill"||name==L"BackgroundStroke";
            const bool possibleAppearance=name==L"TaskListButton"||name==L"Icon"||name==L"RunningIndicator"||name==L"BackgroundElement";
            if(!brushTarget&&!possibleAppearance)return S_OK;
            winrt::Windows::Foundation::IInspectable value{nullptr};
            winrt::check_hresult(diagnostics->GetIInspectableFromHandle(element.Handle,
                reinterpret_cast<::IInspectable**>(winrt::put_abi(value))));
            auto framework=value.try_as<xaml::FrameworkElement>();
            if(!framework||!isTaskbarElement(framework))return S_OK;
            if (!ui) {
                ui = new UiContext(mapping);
                ui->timer = SetTimer(nullptr, 0, 100, tick);
                winrt::check_bool(ui->timer != 0);
            }
            if(possibleAppearance){
                ElementKind kind;
                if(isAppButton(framework))kind=AppButton;
                else if(!hasAppButtonParent(media::VisualTreeHelper::GetParent(framework)))return S_OK;
                else if(name==L"Icon"){
                    if(!framework.try_as<xaml::Controls::Image>())return S_OK;
                    kind=AppIcon;
                }
                else if(name==L"RunningIndicator")kind=AppIndicator;
                else if(name==L"BackgroundElement")kind=AppBackground;
                else return S_OK;
                if(std::none_of(ui->elements.begin(),ui->elements.end(),[&](auto const& item){return item->handle==element.Handle;})){
                    auto target=std::make_shared<ElementTarget>();target->handle=element.Handle;target->element=winrt::make_weak(framework);target->kind=kind;
                    if(kind==AppButton){std::weak_ptr<ElementTarget> weak=target;target->entered=framework.PointerEntered([weak](auto const&,auto const&){if(auto t=weak.lock())t->pointerOver=true;});target->exited=framework.PointerExited([weak](auto const&,auto const&){if(auto t=weak.lock())t->pointerOver=false;});}
                    ui->elements.push_back(target);
                }
                ui->timer=SetTimer(nullptr,ui->timer,100,tick);ui->update();return S_OK;
            }
            auto rectangle = value.try_as<shapes::Rectangle>();
            if(!rectangle)return S_OK;
            if (std::none_of(ui->targets.begin(), ui->targets.end(), [&](Target const& item) { return item.handle == element.Handle; })) {
                ui->targets.push_back(Target{
                    element.Handle,
                    winrt::make_weak(rectangle),
                    rectangle.ReadLocalValue(shapes::Shape::FillProperty()),
                    rectangle.ReadLocalValue(shapes::Shape::StrokeProperty()),
                    rectangle.StrokeThickness(),
                    rectangle.Visibility(),
                    rectangle.Opacity(),
                    nullptr,
                    name == L"BackgroundStroke"
                });
            }
            ui->update();
            return S_OK;
        } catch (...) {
            if (mapping) InterlockedExchange(&mapping->data->attachError, winrt::to_hresult());
            return S_OK;
        }
    }
    HRESULT STDMETHODCALLTYPE OnElementStateChanged(InstanceHandle, VisualElementState, LPCWSTR) noexcept override { return S_OK; }
};

class Factory final : public IClassFactory {
    std::atomic<ULONG> references{1};
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** out) noexcept override {
        if (!out) return E_POINTER;
        *out = nullptr;
        if (iid != __uuidof(IUnknown) && iid != __uuidof(IClassFactory)) return E_NOINTERFACE;
        *out = static_cast<IClassFactory*>(this); AddRef(); return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() noexcept override { return ++references; }
    ULONG STDMETHODCALLTYPE Release() noexcept override { const auto n = --references; if (!n) delete this; return n; }
    HRESULT STDMETHODCALLTYPE LockServer(BOOL) noexcept override { return S_OK; }
    HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown* outer, REFIID iid, void** out) noexcept override {
        if (outer) return CLASS_E_NOAGGREGATION;
        try { auto instance = new Tap; auto result = instance->QueryInterface(iid, out); instance->Release(); return result; }
        catch (...) { return winrt::to_hresult(); }
    }
};

std::mutex controllerMutex;
std::map<DWORD, std::shared_ptr<Mapping>> mappings;

HRESULT connect(DWORD pid, wchar_t const* path, Shared* shared) {
    if (shared->attached == 1) return S_OK;
    HMODULE xamlDll = LoadLibraryExW(L"Windows.UI.Xaml.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!xamlDll) return HRESULT_FROM_WIN32(GetLastError());
    auto initialize = reinterpret_cast<decltype(&InitializeXamlDiagnosticsEx)>(GetProcAddress(xamlDll, "InitializeXamlDiagnosticsEx"));
    if (!initialize) { FreeLibrary(xamlDll); return E_NOTIMPL; }
    HRESULT result = E_FAIL;
    for (unsigned index = 1; index <= 10000; ++index) {
        const auto endpoint = L"VisualDiagConnection" + std::to_wstring(index);
        std::thread([&]() noexcept { result = initialize(endpoint.c_str(), pid, nullptr, path, TapClass, nullptr); }).join();
        if (result != HRESULT_FROM_WIN32(ERROR_NOT_FOUND)) break;
    }
    FreeLibrary(xamlDll);
    return result;
}
}

#pragma comment(linker, "/export:DllGetClassObject")
#pragma comment(linker, "/export:DllCanUnloadNow")
STDAPI DllGetClassObject(REFCLSID clsid, REFIID iid, void** out) {
    if (clsid != TapClass) return CLASS_E_CLASSNOTAVAILABLE;
    try { auto factory = new Factory; const auto result = factory->QueryInterface(iid, out); factory->Release(); return result; }
    catch (...) { return winrt::to_hresult(); }
}
STDAPI DllCanUnloadNow() { return S_FALSE; }

extern "C" __declspec(dllexport) HRESULT WINAPI AtelierApplyBrush(DWORD pid, DWORD material, DWORD color, wchar_t const* dllPath) noexcept {
    try {
        if (!pid || material > Acrylic || !dllPath) return E_INVALIDARG;
        std::lock_guard<std::mutex> guard(controllerMutex);
        auto& mapping = mappings[pid];
        if (!mapping) { auto created = std::make_shared<Mapping>(); created->open(pid, true); mapping = std::move(created); }
        auto* shared = mapping->data;
        if (shared->owner && shared->owner != GetCurrentProcessId()) {
            const auto owner = OpenProcess(SYNCHRONIZE, FALSE, shared->owner);
            if (owner) { const bool alive = WaitForSingleObject(owner, 0) == WAIT_TIMEOUT; CloseHandle(owner); if (alive) return HRESULT_FROM_WIN32(ERROR_BUSY); }
        }
        InterlockedIncrement(&shared->sequence);
        shared->owner = material == System ? 0 : GetCurrentProcessId();
        shared->material = material;
        shared->color = color;
        const LONG sequence = InterlockedIncrement(&shared->sequence);
        const HRESULT result = connect(pid, dllPath, shared);
        if (FAILED(result)) return result;
        const auto deadline = GetTickCount64() + 5000;
        do {
            bool found = false, pending = false;
            HRESULT error = S_OK;
            for (auto const& slot : shared->slots) {
                if (slot.targets > 0) {
                    found = true;
                    if (slot.acknowledged != sequence) pending = true;
                    else if (FAILED(slot.error)) error = slot.error;
                }
            }
            if (found && !pending) return error;
            if (shared->attached < 0) return shared->attachError;
            Sleep(20);
        } while (GetTickCount64() < deadline);
        return FAILED(shared->attachError) ? shared->attachError : HRESULT_FROM_WIN32(ERROR_TIMEOUT);
    } catch (...) { return winrt::to_hresult(); }
}

extern "C" __declspec(dllexport) HRESULT WINAPI AtelierApplyAppearance(DWORD pid,DWORD iconSize,DWORD buttonWidth,LONG buttonRadius,DWORD inactiveOpacity,DWORD indicator,DWORD hoverScale,wchar_t const* dllPath) noexcept {
    try{
        if(!pid||!dllPath||(iconSize&& (iconSize<16||iconSize>64))||(buttonWidth&&(buttonWidth<24||buttonWidth>120))||buttonRadius< -1||buttonRadius>24||inactiveOpacity<30||inactiveOpacity>100||indicator>3||hoverScale<100||hoverScale>150)return E_INVALIDARG;
        std::lock_guard<std::mutex> guard(controllerMutex);
        auto& mapping=mappings[pid];if(!mapping){auto created=std::make_shared<Mapping>();created->open(pid,true);mapping=std::move(created);}
        auto* shared=mapping->data;
        if(shared->appearanceOwner&&shared->appearanceOwner!=GetCurrentProcessId()){
            auto owner=OpenProcess(SYNCHRONIZE,FALSE,shared->appearanceOwner);if(owner){const bool alive=WaitForSingleObject(owner,0)==WAIT_TIMEOUT;CloseHandle(owner);if(alive)return HRESULT_FROM_WIN32(ERROR_BUSY);}
        }
        const bool changed=iconSize||buttonWidth||buttonRadius>=0||inactiveOpacity<100||indicator||hoverScale>100;
        InterlockedIncrement(&shared->sequence);shared->appearanceOwner=changed?GetCurrentProcessId():0;
        shared->iconSize=iconSize;shared->buttonWidth=buttonWidth;shared->buttonRadius=buttonRadius;shared->inactiveOpacity=inactiveOpacity;shared->indicator=indicator;shared->hoverScale=hoverScale;
        const LONG sequence=InterlockedIncrement(&shared->sequence);
        auto result=connect(pid,dllPath,shared);if(FAILED(result))return result;
        const auto deadline=GetTickCount64()+5000;
        do{
            bool found=false,pending=false;
            for(auto const& slot:shared->slots){
                if(slot.buttons>0){found=true;if(FAILED(slot.error))return slot.error;if(slot.acknowledged!=sequence)pending=true;}
            }
            if(found&&!pending)return S_OK;
            if(shared->attached<0)return shared->attachError;Sleep(20);
        }while(GetTickCount64()<deadline);
        return HRESULT_FROM_WIN32(ERROR_NOT_FOUND);
    }catch(...){return winrt::to_hresult();}
}
extern "C" __declspec(dllexport) HRESULT WINAPI AtelierReadAppearance(DWORD pid,LONG* values,DWORD count) noexcept {
    if(!values||(count!=4&&count!=8&&count!=10))return E_INVALIDARG;
    try{std::lock_guard<std::mutex> guard(controllerMutex);auto it=mappings.find(pid);if(it==mappings.end())return HRESULT_FROM_WIN32(ERROR_NOT_FOUND);
        std::fill(values,values+count,0);for(auto const& slot:it->second->data->slots){values[0]+=slot.buttons;values[1]+=slot.icons;if(slot.buttonPixels)values[3]=slot.buttonPixels;if(slot.iconPixels){values[2]=slot.iconPixels;if(count>=8){values[4]=slot.iconWidth;values[5]=slot.iconMinimum;values[6]=slot.iconMaximum;values[7]=slot.sourcePixels;}}if(count==10){values[8]+=slot.highResolutionIcons;values[9]+=slot.upscaledIcons;}}return S_OK;
    }catch(...){return winrt::to_hresult();}
}
extern "C" __declspec(dllexport) HRESULT WINAPI AtelierRepairIcons(DWORD pid,wchar_t const* dllPath) noexcept{
    try{
        if(!pid||!dllPath)return E_INVALIDARG;std::lock_guard<std::mutex> guard(controllerMutex);
        auto& mapping=mappings[pid];if(!mapping){auto created=std::make_shared<Mapping>();created->open(pid,true);mapping=std::move(created);}auto* shared=mapping->data;
        if(shared->appearanceOwner&&shared->appearanceOwner!=GetCurrentProcessId()){auto owner=OpenProcess(SYNCHRONIZE,FALSE,shared->appearanceOwner);if(owner){bool alive=WaitForSingleObject(owner,0)==WAIT_TIMEOUT;CloseHandle(owner);if(alive)return HRESULT_FROM_WIN32(ERROR_BUSY);}}
        InterlockedIncrement(&shared->sequence);shared->appearanceOwner=0;shared->iconSize=0;shared->buttonWidth=0;shared->buttonRadius=-1;shared->inactiveOpacity=100;shared->indicator=0;shared->hoverScale=100;shared->repairEpoch=shared->sequence+1;const LONG repair=shared->repairEpoch;InterlockedIncrement(&shared->sequence);
        HRESULT result=connect(pid,dllPath,shared);if(FAILED(result)){shared->repairEpoch=0;return result;}
        auto deadline=GetTickCount64()+5000;LONG lastCount=-1;ULONGLONG stable=GetTickCount64();bool done=false;
        while(GetTickCount64()<deadline){LONG count=0;bool pending=false;for(auto const& slot:shared->slots){count+=slot.buttons+slot.icons;if(slot.buttons>0&&slot.repairAcknowledged!=repair)pending=true;}if(count!=lastCount){lastCount=count;stable=GetTickCount64();}if(count>1&&!pending&&GetTickCount64()-stable>250){done=true;break;}Sleep(20);}
        InterlockedIncrement(&shared->sequence);shared->repairEpoch=0;InterlockedIncrement(&shared->sequence);return done?S_OK:HRESULT_FROM_WIN32(ERROR_TIMEOUT);
    }catch(...){return winrt::to_hresult();}
}
extern "C" __declspec(dllexport) HRESULT WINAPI AtelierReadIconDescription(DWORD pid,wchar_t* buffer,DWORD capacity) noexcept{
    if(!buffer||capacity<2)return E_INVALIDARG;try{std::lock_guard<std::mutex> guard(controllerMutex);auto it=mappings.find(pid);if(it==mappings.end())return E_FAIL;buffer[0]=0;for(auto const& slot:it->second->data->slots){if(slot.description[0])wcsncpy_s(buffer,capacity,slot.description,_TRUNCATE);}return S_OK;}catch(...){return winrt::to_hresult();}
}
