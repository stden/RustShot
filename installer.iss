[Setup]
AppName=RustShot
AppVersion={{VERSION}}
AppPublisher=RustShot Contributors
AppPublisherURL=https://github.com/stden/RustShot
AppSupportURL=https://github.com/stden/RustShot/issues
DefaultDirName={autopf}\RustShot
DefaultGroupName=RustShot
OutputDir=Output
OutputBaseFilename=RustShotSetup
Compression=lzma
SolidCompression=yes
; Install without admin rights so autorun works in current user context
PrivilegesRequired=lowest
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Files]
Source: "target\release\rustshot.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\RustShot"; Filename: "{app}\rustshot.exe"; Comment: "Screenshot and screen recording utility"
Name: "{group}\Uninstall RustShot"; Filename: "{uninstallexe}"

[Registry]
; Add to Windows autorun so the tray icon appears on every login
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "RustShot"; ValueData: """{app}\rustshot.exe"""; Flags: uninsdeletevalue

[Run]
; Launch immediately after install so the tray icon appears right away
Filename: "{app}\rustshot.exe"; Description: "Launch RustShot (system tray)"; Flags: nowait postinstall skipifsilent
