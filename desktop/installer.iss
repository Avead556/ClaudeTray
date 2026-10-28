#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif
#ifndef Binary
  #define Binary "target\release\claudetray.exe"
#endif

[Setup]
AppId={{6F0C2B4E-8D1A-4C7B-9E3F-2A5D7C1B9E40}
AppName=ClaudeTray
AppVersion={#AppVersion}
AppPublisher=ClaudeTray
DefaultDirName={localappdata}\Programs\ClaudeTray
DefaultGroupName=ClaudeTray
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=dist
OutputBaseFilename=ClaudeTray-windows-setup
UninstallDisplayIcon={app}\claudetray.exe
ChangesEnvironment=yes
CloseApplications=force
WizardStyle=modern
Compression=lzma2
SolidCompression=yes

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "ru"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "uk"; MessagesFile: "compiler:Languages\Ukrainian.isl"
Name: "de"; MessagesFile: "compiler:Languages\German.isl"
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "it"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "pl"; MessagesFile: "compiler:Languages\Polish.isl"
Name: "tr"; MessagesFile: "compiler:Languages\Turkish.isl"
Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "autostart"; Description: "Open at login"
Name: "path"; Description: "Add the claudetray command to PATH"

[Files]
Source: "{#Binary}"; DestDir: "{app}"; DestName: "claudetray.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\ClaudeTray"; Filename: "{app}\claudetray.exe"

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "ClaudeTray"; ValueData: "{app}\claudetray.exe"; Tasks: autostart; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: path; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\claudetray.exe"; Description: "{cm:LaunchProgram,ClaudeTray}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/F /IM claudetray.exe"; Flags: runhidden; RunOnceId: "StopClaudeTray"

[UninstallDelete]
Type: filesandordirs; Name: "{app}"

[Code]
function UserPath(): String;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Result) then
    Result := '';
end;

function NeedsAddPath(Dir: String): Boolean;
begin
  Result := Pos(';' + Uppercase(Dir) + ';', ';' + Uppercase(UserPath()) + ';') = 0;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Path, Dir: String;
  Index: Integer;
begin
  if CurUninstallStep <> usPostUninstall then
    Exit;
  Path := ';' + UserPath() + ';';
  Dir := ';' + ExpandConstant('{app}') + ';';
  Index := Pos(Uppercase(Dir), Uppercase(Path));
  if Index = 0 then
    Exit;
  Delete(Path, Index, Length(Dir) - 1);
  Path := Copy(Path, 2, Length(Path) - 2);
  RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', Path);
end;
