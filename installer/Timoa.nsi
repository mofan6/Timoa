Unicode true
!include "MUI2.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "FileFunc.nsh"

!ifndef APP_EXE
  !define APP_EXE "..\target\release\Timoa.exe"
!endif
!ifndef PACKAGE_DIR
  !define PACKAGE_DIR "..\dist"
!endif
Name "Timoa"
OutFile "${PACKAGE_DIR}\Timoa-Setup.exe"
InstallDir "$LOCALAPPDATA\Programs\Timoa"
InstallDirRegKey HKCU "Software\TIMO" "InstallDir"
RequestExecutionLevel user
SetCompressor zlib
SetDatablockOptimize on
ManifestDPIAware true
Icon "..\assets\timoa.ico"
UninstallIcon "..\assets\timoa.ico"
BrandingText "Timoa"
VIProductVersion "1.0.1.0"
VIAddVersionKey "ProductName" "Timoa"
VIAddVersionKey "ProductVersion" "1.0.1"
VIAddVersionKey "FileDescription" "Timoa 安装程序"
VIAddVersionKey "FileVersion" "1.0.1.0"
VIAddVersionKey "LegalCopyright" ""

!define MUI_ABORTWARNING
!define MUI_ICON "..\assets\timoa.ico"
!define MUI_UNICON "..\assets\timoa.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\Timoa.exe"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!define MUI_FINISHPAGE_RUN_TEXT "运行 Timoa"
!define MUI_WELCOMEPAGE_TITLE "欢迎安装 Timoa"
!define MUI_WELCOMEPAGE_TEXT "专注计时、闹钟计划与每周安排。$\r$\n$\r$\n安装在当前用户目录，无需额外运行库。"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"

!macro RunningCheck Prefix
Function ${Prefix}CheckRunning
  System::Call 'kernel32::OpenMutexW(i 0x100000, i 0, w "Local\TIMO.Native.SingleInstance.v1") p.r0'
  StrCmp $0 0 available
  System::Call 'kernel32::CloseHandle(p r0)'
  MessageBox MB_OK|MB_ICONINFORMATION "请先从系统托盘退出 Timoa，然后重新运行安装或卸载程序。" /SD IDOK
  Abort
  available:
FunctionEnd
!macroend
!insertmacro RunningCheck ""
!insertmacro RunningCheck "un."

Function .onInit
  ${IfNot} ${RunningX64}
  ${OrIfNot} ${AtLeastWin10}
    MessageBox MB_OK|MB_ICONSTOP "Timoa 需要 64 位 Windows 10 或更高版本。" /SD IDOK
    Abort
  ${EndIf}
  SetShellVarContext current
  SetRegView 64
FunctionEnd

Function un.onInit
  SetShellVarContext current
  SetRegView 64
  Call un.CheckRunning
FunctionEnd

Section "Timoa" Main
  SectionIn RO
  Call CheckRunning
  SetOutPath "$INSTDIR"
  SetOverwrite on
  File /oname=Timoa.exe "${APP_EXE}"
  Delete "$INSTDIR\TIMO.exe"
  Delete "$SMPROGRAMS\TIMO.lnk"
  Delete "$DESKTOP\TIMO.lnk"
  ; Keep an existing startup preference while moving from a portable/preview path.
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "TIMO"
  StrCmp $0 "" startup_unchanged
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "TIMO" '$\"$INSTDIR\Timoa.exe$\" --startup'
  startup_unchanged:
  File /oname=LICENSE.txt "..\LICENSE"
  File "..\NOTICE.txt"
  SetOutPath "$INSTDIR\licenses"
  File "..\licenses\Apache-2.0.txt"
  File "..\licenses\NSIS.txt"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\TIMO" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "DisplayName" "Timoa"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "DisplayVersion" "1.0.1"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "Publisher" "MOTAN"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "DisplayIcon" "$INSTDIR\Timoa.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO" "EstimatedSize" $0
  CreateShortcut "$SMPROGRAMS\Timoa.lnk" "$INSTDIR\Timoa.exe"
SectionEnd

Section "桌面快捷方式" Desktop
  CreateShortcut "$DESKTOP\Timoa.lnk" "$INSTDIR\Timoa.exe"
SectionEnd

Section "Uninstall"
  Call un.CheckRunning
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "TIMO"
  StrLen $1 '"$INSTDIR\Timoa.exe"'
  StrCpy $2 $0 $1
  StrCmp $2 '"$INSTDIR\Timoa.exe"' 0 +2
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "TIMO"
  Delete "$INSTDIR\Timoa.exe"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\NOTICE.txt"
  Delete "$INSTDIR\licenses\Apache-2.0.txt"
  Delete "$INSTDIR\licenses\NSIS.txt"
  RMDir "$INSTDIR\licenses"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Timoa.lnk"
  Delete "$DESKTOP\Timoa.lnk"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\TIMO"
  DeleteRegKey HKCU "Software\TIMO"
  ; Reminder data and custom audio remain in AppData for a future reinstall.
SectionEnd
