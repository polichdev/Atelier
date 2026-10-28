#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <windows.ui.xaml.hosting.desktopwindowxamlsource.h>
#undef GetCurrentTime
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Foundation.Collections.h>
#include <winrt/Windows.UI.h>
#include <winrt/Windows.UI.Xaml.h>
#include <winrt/Windows.UI.Xaml.Controls.h>
#include <winrt/Windows.UI.Xaml.Controls.Primitives.h>
#include <winrt/Windows.UI.Xaml.Hosting.h>
#include <winrt/Windows.UI.Xaml.Media.h>
#include <winrt/Windows.UI.Xaml.Media.Imaging.h>
#include <winrt/Windows.UI.Xaml.Data.h>
#include <winrt/Windows.UI.Xaml.Shapes.h>
#include <cstdio>
#include <string>
#include <thread>
#include <atomic>
#pragma comment(lib,"advapi32.lib")

namespace x = winrt::Windows::UI::Xaml;
using Apply = HRESULT(WINAPI*)(DWORD, DWORD, DWORD, wchar_t const*);
using Appearance=HRESULT(WINAPI*)(DWORD,DWORD,DWORD,LONG,DWORD,DWORD,DWORD,wchar_t const*);
constexpr UINT Inspect = WM_APP + 10;
thread_local x::Shapes::Rectangle background{nullptr};
thread_local x::Shapes::Rectangle border{nullptr};
thread_local x::Controls::Button appButton{nullptr};
thread_local x::Controls::Image appIcon{nullptr};
thread_local x::Shapes::Rectangle runningIndicator{nullptr};
thread_local x::Controls::Border highlight{nullptr};
thread_local x::Controls::Slider widthSource{nullptr};
std::atomic<HWND> window = nullptr;
std::atomic<HRESULT> hostError = S_OK;

LRESULT CALLBACK windowProc(HWND hwnd, UINT message, WPARAM which, LPARAM data) noexcept {
    try {
        if (message == WM_DESTROY) { PostQuitMessage(0); return 0; }
        if(message==Inspect&&which>=10&&appButton){
            if(which==10)return static_cast<LRESULT>(appIcon.Width()*100+.5);
            if(which==11)return static_cast<LRESULT>(appButton.Width()*100+.5);
            if(which==12)return static_cast<LRESULT>(highlight.CornerRadius().TopLeft*100+.5);
            if(which==13)return runningIndicator.Visibility()==x::Visibility::Collapsed?1:0;
            if(which==14)return static_cast<LRESULT>(appButton.Opacity()*100+.5);
            if(which==15){widthSource.Value(26);return 0;}
            if(which==16){widthSource.Value(24);return 0;}
            if(which==17){auto transform=appIcon.RenderTransform().try_as<x::Media::ScaleTransform>();return transform?static_cast<LRESULT>(transform.ScaleX()*100+.5):100;}
        }
        if (message == Inspect && background) {
            const auto brush = (which == 2 ? border : background).Fill();
            if (auto solid = brush.try_as<x::Media::SolidColorBrush>()) {
                const auto color = solid.Color();
                if (which == 0) return color.A == 0 ? 1 : 2;
                return color.R | (color.G << 8) | (color.B << 16) | (static_cast<DWORD>(color.A) << 24);
            }
            if (auto acrylic = brush.try_as<x::Media::AcrylicBrush>()) {
                if (which == 0) return 3;
                const auto color = acrylic.TintColor();
                return color.R | (color.G << 8) | (color.B << 16) | (static_cast<DWORD>(acrylic.TintOpacity() * 255 + .5) << 24);
            }
            return -2;
        }
    } catch (...) { return -1; }
    return DefWindowProcW(hwnd, message, which, data);
}
DWORD inspect(WPARAM which) {
    DWORD_PTR result = 0;
    winrt::check_bool(SendMessageTimeoutW(window.load(), Inspect, which, 0, SMTO_ABORTIFHUNG, 2000, &result) != 0);
    return static_cast<DWORD>(result);
}
void expect(bool valid, char const* description) {
    if (!valid) { std::printf("FAIL: %s\n", description); winrt::throw_hresult(E_FAIL); }
    std::printf("PASS: %s\n", description);
}
int wmain(int argc, wchar_t** argv) {
    if (argc < 2) return 2;
    if(std::wstring(argv[1])==L"--position"){
        const auto tray=FindWindowW(L"Shell_TrayWnd",nullptr);if(!tray)return 8;
        RECT before{};GetWindowRect(tray,&before);MONITORINFO mi{sizeof(mi)};GetMonitorInfoW(MonitorFromWindow(tray,MONITOR_DEFAULTTONEAREST),&mi);
        const int original=(before.right-before.left>before.bottom-before.top)?(before.top<=mi.rcMonitor.top+1?1:3):(before.left<=mi.rcMonitor.left+1?0:2);
        DWORD oldValue=0,oldSize=sizeof(oldValue);const wchar_t* key=L"Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced";
        const bool existed=RegGetValueW(HKEY_CURRENT_USER,key,L"TaskbarLocation",RRF_RT_REG_DWORD,nullptr,&oldValue,&oldSize)==ERROR_SUCCESS;
        DWORD_PTR result=0;SendMessageTimeoutW(tray,0x5CA,6,1,SMTO_ABORTIFHUNG,2000,&result);Sleep(400);
        RECT after{};GetWindowRect(tray,&after);const bool moved=after.top==mi.rcMonitor.top&&after.right-after.left>after.bottom-after.top;
        std::printf("Native move to top: %s; before=(%ld,%ld,%ld,%ld), after=(%ld,%ld,%ld,%ld)\n",moved?"applied":"not supported",before.left,before.top,before.right,before.bottom,after.left,after.top,after.right,after.bottom);
        SendMessageTimeoutW(tray,0x5CA,6,original,SMTO_ABORTIFHUNG,2000,&result);Sleep(400);
        HKEY opened=nullptr;if(RegOpenKeyExW(HKEY_CURRENT_USER,key,0,KEY_SET_VALUE,&opened)==ERROR_SUCCESS){if(existed)RegSetValueExW(opened,L"TaskbarLocation",0,REG_DWORD,reinterpret_cast<BYTE*>(&oldValue),sizeof(oldValue));else RegDeleteValueW(opened,L"TaskbarLocation");RegCloseKey(opened);}
        return moved?0:9;
    }
    const bool client = std::wstring(argv[1]) == L"--client";
    const bool nativeTest=std::wstring(argv[1])==L"--native";
    if((client||nativeTest)&&argc<4)return 2;
    const wchar_t* dllPath = (client||nativeTest) ? argv[3] : argv[1];
    HMODULE dll = LoadLibraryW(dllPath);
    if (!dll) return 3;
    const auto apply = reinterpret_cast<Apply>(GetProcAddress(dll, "AtelierApplyBrush"));
    const auto appearance=reinterpret_cast<Appearance>(GetProcAddress(dll,"AtelierApplyAppearance"));
    if (!apply||!appearance) return 4;
    if(nativeTest){
        const DWORD pid=std::stoul(argv[2]);
        using Read=HRESULT(WINAPI*)(DWORD,LONG*,DWORD);
        const auto read=reinterpret_cast<Read>(GetProcAddress(dll,"AtelierReadAppearance"));if(!read)return 7;
        int code=0;
        try{
            const auto repair=reinterpret_cast<HRESULT(WINAPI*)(DWORD,wchar_t const*)>(GetProcAddress(dll,"AtelierRepairIcons"));
            if(repair){winrt::check_hresult(repair(pid,dllPath));expect(true,"Legacy scaled icon visuals are cleared before testing");}
            winrt::check_hresult(appearance(pid,0,0,-1,100,0,100,dllPath));Sleep(300);
            LONG before[4]{};winrt::check_hresult(read(pid,before,4));
            expect(before[0]>0&&before[1]>0,"Real Explorer taskbar buttons and icons are found");
            winrt::check_hresult(appearance(pid,20,32,6,100,0,100,dllPath));Sleep(400);
            LONG after[10]{};winrt::check_hresult(read(pid,after,10));
            std::printf("Explorer targets: %ld buttons, %ld icons; measured icon=%ld/100 px, button=%ld/100 px\n",after[0],after[1],after[2],after[3]);
            std::printf("Icon Width=%ld, Min=%ld, Max=%ld, bitmap source=%ld pixels\n",after[4],after[5],after[6],after[7]);
            const auto describe=reinterpret_cast<HRESULT(WINAPI*)(DWORD,wchar_t*,DWORD)>(GetProcAddress(dll,"AtelierReadIconDescription"));if(describe){wchar_t text[1024]{};describe(pid,text,1024);std::wprintf(L"Icon identity: %s\n",text);}
            std::printf("High-resolution icons: %ld; upscaled low-resolution icons: %ld\n",after[8],after[9]);
            expect(after[2]>=1600&&after[2]<=2000&&after[3]==3200,"Real Explorer icons fit the requested size and clickable buttons use the requested width");
            expect(after[8]>0&&after[9]==0,"Real Explorer uses high-resolution icons without enlarging small cached images");
            winrt::check_hresult(appearance(pid,0,0,-1,100,0,100,dllPath));Sleep(400);
            LONG restored[4]{};winrt::check_hresult(read(pid,restored,4));
            expect(restored[2]==before[2]&&restored[3]==before[3],"Real Explorer original icon and button sizes are restored");
        }catch(winrt::hresult_error const& error){std::printf("ERROR: 0x%08lx\n",static_cast<HRESULT>(error.code()));code=1;}
        catch(...){std::printf("ERROR: unexpected native test failure\n");code=1;}
        appearance(pid,0,0,-1,100,0,100,dllPath);return code;
    }
    if (client) {
        const HRESULT result = apply(std::stoul(argv[2]), 1, 0, dllPath);
        if (FAILED(result)) { std::printf("Child apply: 0x%08lx\n", result); return 5; }
        if(FAILED(appearance(std::stoul(argv[2]),18,28,2,80,1,100,dllPath)))return 6;
        Sleep(400);
        return 0;
    }
    std::thread host([]() noexcept {
        try {
            winrt::init_apartment(winrt::apartment_type::single_threaded);
            auto manager = x::Hosting::WindowsXamlManager::InitializeForCurrentThread();
            WNDCLASSW wc{}; wc.lpfnWndProc = windowProc; wc.hInstance = GetModuleHandleW(nullptr); wc.lpszClassName = L"AtelierBrushProbe";
            RegisterClassW(&wc);
            const auto hwnd = CreateWindowExW(0, wc.lpszClassName, L"Atelier brush integration probe", WS_OVERLAPPEDWINDOW,
                80, 80, 400, 200, nullptr, nullptr, wc.hInstance, nullptr);
            winrt::check_bool(hwnd != nullptr);
            x::Hosting::DesktopWindowXamlSource source;
            auto native = source.as<IDesktopWindowXamlSourceNative>();
            winrt::check_hresult(native->AttachToWindow(hwnd));
            HWND child = nullptr; winrt::check_hresult(native->get_WindowHandle(&child));
            SetWindowPos(child, nullptr, 0, 0, 380, 150, SWP_SHOWWINDOW);
            x::Controls::Grid root; root.Name(L"TaskbarFrame");
            background = x::Shapes::Rectangle(); background.Name(L"BackgroundFill");
            background.Fill(x::Media::SolidColorBrush(winrt::Windows::UI::Color{255, 18, 52, 86}));
            border = x::Shapes::Rectangle(); border.Name(L"BackgroundStroke"); border.Height(1);
            border.Fill(x::Media::SolidColorBrush(winrt::Windows::UI::Color{255, 100, 120, 140}));
            appButton=x::Controls::Button();appButton.Name(L"TaskListButton");appButton.Width(44);
            x::Controls::Grid panel;panel.Name(L"IconPanel");
            appIcon=x::Controls::Image();appIcon.Name(L"Icon");appIcon.Width(24);appIcon.Height(24);
            appIcon.Source(x::Media::Imaging::WriteableBitmap(128,128));widthSource=x::Controls::Slider();widthSource.Maximum(100);widthSource.Value(24);x::Data::Binding widthBinding;widthBinding.Source(widthSource);widthBinding.Path(x::PropertyPath(L"Value"));appIcon.SetBinding(x::FrameworkElement::WidthProperty(),widthBinding);
            runningIndicator=x::Shapes::Rectangle();runningIndicator.Name(L"RunningIndicator");runningIndicator.Width(12);runningIndicator.Height(3);
            highlight=x::Controls::Border();highlight.Name(L"BackgroundElement");highlight.CornerRadius({4,4,4,4});
            panel.Children().Append(highlight);panel.Children().Append(appIcon);panel.Children().Append(runningIndicator);appButton.Content(panel);
            root.Children().Append(background); root.Children().Append(border);root.Children().Append(appButton); source.Content(root);
            ShowWindow(hwnd, SW_SHOWNOACTIVATE); window = hwnd;
            MSG message{};
            while (GetMessageW(&message, nullptr, 0, 0) > 0) { TranslateMessage(&message); DispatchMessageW(&message); }
            background = nullptr; border = nullptr; source.Close(); manager.Close();
        } catch (...) { hostError = winrt::to_hresult(); }
    });
    int result = 0;
    try {
        for (int i = 0; !window && SUCCEEDED(hostError) && i < 250; ++i) Sleep(20);
        winrt::check_hresult(hostError.load()); expect(window != nullptr, "XAML fixture opened");
        winrt::check_hresult(apply(GetCurrentProcessId(), 1, 0, dllPath));
        expect(inspect(0) == 1 && inspect(2) == 0, "Clear removes background and border fill");
        winrt::check_hresult(apply(GetCurrentProcessId(), 3, 0xb4563412, dllPath));
        expect(inspect(0) == 1 && inspect(2) == 0, "Acrylic removes background and border fill");
        winrt::check_hresult(apply(GetCurrentProcessId(), 2, 0, dllPath));
        expect(inspect(0) == 1 && inspect(2) == 0, "Blur removes background and border fill");
        winrt::check_hresult(appearance(GetCurrentProcessId(),20,32,6,65,3,125,dllPath));
        expect(inspect(10)==2000&&inspect(11)==3200,"Native icon size and taskbar button width change");
        expect(inspect(12)==600&&inspect(13)==1&&inspect(14)==65,"Native button corners, running indicator and opacity change");
        winrt::check_hresult(apply(GetCurrentProcessId(), 0, 0, dllPath));
        expect(inspect(10)==2000,"Default material does not reset an independently owned native icon style");
        winrt::check_hresult(appearance(GetCurrentProcessId(),0,0,-1,100,0,100,dllPath));
        expect(inspect(10)==2400&&inspect(11)==4400&&inspect(12)==400&&inspect(13)==0&&inspect(14)==100,"Reset restores original native properties");
        expect(inspect(17)==100,"Reset removes the custom icon render transform");inspect(15);Sleep(30);expect(inspect(10)==2600,"Reset reconnects the original live XAML width binding");inspect(16);Sleep(30);
        const auto repair=reinterpret_cast<HRESULT(WINAPI*)(DWORD,wchar_t const*)>(GetProcAddress(dll,"AtelierRepairIcons"));
        expect(repair!=nullptr,"Repair entry point is available");
        winrt::check_hresult(repair(GetCurrentProcessId(),dllPath));
        inspect(15);Sleep(30);expect(inspect(10)==2600&&inspect(11)==4400,"Repair preserves Explorer's original binding and button width");inspect(16);Sleep(30);
        expect(appearance(GetCurrentProcessId(),1,0,-1,100,0,100,dllPath)==E_INVALIDARG,"Invalid native appearance is rejected");
        expect(inspect(1) == 0xff563412 && inspect(2) == 0xff8c7864, "Default restores original background and border");
        expect(apply(GetCurrentProcessId(), 99, 0, dllPath) == E_INVALIDARG, "Invalid material is rejected");
        wchar_t exe[MAX_PATH]{}; GetModuleFileNameW(nullptr, exe, MAX_PATH);
        std::wstring command = L"\"" + std::wstring(exe) + L"\" --client " + std::to_wstring(GetCurrentProcessId()) + L" \"" + dllPath + L"\"";
        STARTUPINFOW startup{sizeof(startup)}; PROCESS_INFORMATION process{};
        winrt::check_bool(CreateProcessW(nullptr, command.data(), nullptr, nullptr, FALSE, 0, nullptr, nullptr, &startup, &process));
        const auto wait = WaitForSingleObject(process.hProcess, 10000);
        DWORD code = 1; GetExitCodeProcess(process.hProcess, &code); CloseHandle(process.hThread); CloseHandle(process.hProcess);
        expect(wait == WAIT_OBJECT_0 && code == 0, "Second controller can acquire a restored session");
        for (int i = 0; i < 50 && inspect(0) != 2; ++i) Sleep(20);
        expect(inspect(1) == 0xff563412 && inspect(2) == 0xff8c7864, "Controller exit automatically restores original brushes");
        for(int i=0;i<50&&inspect(10)!=2400;++i)Sleep(20);
        expect(inspect(10)==2400&&inspect(11)==4400&&inspect(12)==400,"Controller exit also restores native icon and button styles");
        expect(inspect(17)==100,"Controller exit leaves no scaling transform behind");
    } catch (winrt::hresult_error const& error) { std::printf("ERROR: 0x%08lx\n", static_cast<HRESULT>(error.code())); result = 1; }
      catch (...) { std::printf("ERROR: unexpected failure\n"); result = 1; }
    if (window) PostMessageW(window, WM_CLOSE, 0, 0);
    if (host.joinable()) host.join();
    return result;
}
