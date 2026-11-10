#ifndef AppVersion
  #error AppVersion must be passed to the Inno Setup compiler
#endif
#ifndef StageDir
  #error StageDir must be passed to the Inno Setup compiler
#endif
#ifndef OutputBaseFilename
  #error OutputBaseFilename must be passed to the Inno Setup compiler
#endif

[Setup]
AppId={{D8EA0D29-8BA8-4861-9EF0-21A8062D1B4B}
AppName=MorseRunner
AppVersion={#AppVersion}
AppPublisher=Vladimir Vecgailis
AppPublisherURL=https://github.com/v2b1n/MorseRunner
AppSupportURL=https://github.com/v2b1n/MorseRunner/issues
DefaultDirName={localappdata}\Programs\MorseRunner
DefaultGroupName=MorseRunner
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
UninstallDisplayIcon={app}\morse-runner.exe
OutputDir=..\dist
OutputBaseFilename={#OutputBaseFilename}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
Uninstallable=yes
CloseApplications=no

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"; Flags: unchecked

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\MorseRunner"; Filename: "{app}\morse-runner.exe"
Name: "{autodesktop}\MorseRunner"; Filename: "{app}\morse-runner.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\morse-runner.exe"; Description: "Launch MorseRunner"; Flags: postinstall nowait skipifsilent

