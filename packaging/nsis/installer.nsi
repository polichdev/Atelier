!include "MUI2.nsh"
!include "FileFunc.nsh"

Name "Atelier"
OutFile "Atelier-Setup.exe"
InstallDir "$LOCALAPPDATA\Atelier"
RequestExecutionLevel user

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "target\release\atelier-core.exe"

  WriteRegStr HKCU "Software\Atelier" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Atelier" "$INSTDIR\atelier-core.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Atelier" "DisplayName" "Atelier"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Atelier" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Atelier" "DisplayVersion" "0.1.0"
SectionEnd

Section "Uninstall"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Atelier"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Atelier"
  DeleteRegKey HKCU "Software\Atelier"

  Delete "$INSTDIR\atelier-core.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
SectionEnd
