; Inno Setup Script for RemoteDesk Free
; Comprehensive Freeware Windows Installation Package

#define MyAppName "RemoteDesk Free"
#define MyAppVersion "1.1.0.1"
#define MyAppPublisher "Suresh Mamidala"
#define MyAppCompany "Freeware | Suresh Mamidala"
#define MyAppURL "https://github.com/remotedesk-free/remotedesk-free"
#define MyAppExeName "RemoteDesk-Free.exe"
#define MyServerExeName "RemoteDesk-Server.exe"

[Setup]
AppId={{8B4F9231-92D0-4F25-A2B3-73F92C7F1944}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
VersionInfoVersion=1.1.0.1
VersionInfoCompany=Freeware | Suresh Mamidala
VersionInfoDescription=RemoteDesk Free Windows Enterprise Application
VersionInfoTextVersion=1.1.0.1
VersionInfoCopyright=Copyright (C) 2026 Suresh Mamidala
DefaultDirName={commonpf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
LicenseFile=LICENSE
PrivilegesRequired=admin
CloseApplications=yes
RestartApplications=no
OutputDir=dist
OutputBaseFilename=RemoteDesk-Free-Setup-v1.1.0.1
Compression=lzma2/max
SolidCompression=no
WizardStyle=modern
ArchitecturesAllowed=x64compatible arm64
ArchitecturesInstallIn64BitMode=x64compatible arm64
DisableProgramGroupPage=auto
UninstallDisplayIcon={app}\{#MyAppExeName}
SetupIconFile=icon.ico

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "autostart"; Description: "Start RemoteDesk Free automatically when Windows starts (for Unattended Access)"; GroupDescription: "System Integration:"; Flags: unchecked

[Files]
Source: "RemoteDesk-Free.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "RemoteDesk-Server.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "remotedesk_native.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "RemoteDesk-Free-User-Guide.pdf"; DestDir: "{app}"; Flags: ignoreversion
Source: "icon.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "icon.png"; DestDir: "{app}"; Flags: ignoreversion
Source: "libunwind.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "THIRD_PARTY_LICENSES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "docs\*"; DestDir: "{app}\docs"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\icon.ico"; WorkingDir: "{app}"
Name: "{group}\{#MyAppName} Server"; Filename: "{app}\{#MyServerExeName}"; IconFilename: "{app}\icon.ico"; WorkingDir: "{app}"
Name: "{group}\{#MyAppName} User Guide (PDF)"; Filename: "{app}\RemoteDesk-Free-User-Guide.pdf"; WorkingDir: "{app}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"; IconFilename: "{app}\icon.ico"; WorkingDir: "{app}"
Name: "{commondesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\icon.ico"; WorkingDir: "{app}"; Tasks: desktopicon
Name: "{commonstartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\icon.ico"; WorkingDir: "{app}"; Parameters: "--daemon"; Tasks: autostart

[Run]
Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent

[Code]
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/F /IM RemoteDesk-Free.exe /T', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM RemoteDesk-Server.exe /T', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := True;
end;

function InitializeUninstall(): Boolean;
var
  ResultCode: Integer;
begin
  Exec('taskkill.exe', '/F /IM RemoteDesk-Free.exe /T', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec('taskkill.exe', '/F /IM RemoteDesk-Server.exe /T', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Result := True;
end;
