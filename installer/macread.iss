; Instalador do macread — SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA
; Compilar com o Inno Setup 6.1 ou mais novo:  ISCC.exe installer\macread.iss
; O instalador não precisa de nada pré-instalado: verifica o driver Dokan 2 (montagem como
; unidade) e, se faltar, baixa-o da página oficial e instala.

#define AppName "macread"
#define AppVersion "1.1.0"
#define Company "SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA"
#define CompanyNif "5002359936"
#define CompanyPhone "932693623"
#define CompanySite "https://sudomakes.com"
#define Repo "https://github.com/arturjose0/macread"
#define DokanVersion "2.3.1.1000"
#define DokanUrl "https://github.com/dokan-dev/dokany/releases/download/v2.3.1.1000/Dokan_x64.msi"
#define DokanSha256 "69ff8cb37bfec3a75921c85ffd1c6370b50a9ec4ecef2cf3a009d488dcbf5465"
#define AppComments "Lê e copia arquivos de discos Mac (APFS, HFS+) e Linux (ext2/3/4, LVM) no Windows."

[Setup]
AppId={{B7D3F0C2-5A6E-4C2B-9F0E-6D1A2B3C4D5E}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#Company}
AppPublisherURL={#CompanySite}
AppSupportURL={#Repo}/issues
AppUpdatesURL={#Repo}/releases
AppContact=Tel. {#CompanyPhone} · NIF {#CompanyNif}
AppComments={#AppComments}
VersionInfoCompany={#Company}
VersionInfoDescription=Instalador do macread
VersionInfoProductName={#AppName}
VersionInfoVersion={#AppVersion}
DefaultDirName={autopf}\macread
DefaultGroupName=macread
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\macread.ico
UninstallDisplayName=macread {#AppVersion} (SUDOMAKE)
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\dist
OutputBaseFilename=macread-setup-{#AppVersion}
SetupIconFile=macread.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\LICENSE
InfoBeforeFile=sobre.txt
ShowLanguageDialog=auto
ChangesEnvironment=yes

[Languages]
Name: "pt"; MessagesFile: "compiler:Languages\Portuguese.isl"
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Messages]
pt.WelcomeLabel2=Este assistente vai instalar o [name/ver] neste computador.%n%nO macread lê e copia ficheiros de discos de Mac (APFS, HFS+) e Linux (ext2/3/4, LVM) directamente no Windows, sem drivers, e pode montá-los como uma unidade.%n%nDesenvolvido por {#Company}%nNIF {#CompanyNif} · Tel. {#CompanyPhone} · {#CompanySite}
ptbr.WelcomeLabel2=Este assistente vai instalar o [name/ver] neste computador.%n%nO macread lê e copia arquivos de discos de Mac (APFS, HFS+) e Linux (ext2/3/4, LVM) diretamente no Windows, sem drivers, e pode montá-los como uma unidade.%n%nDesenvolvido por {#Company}%nNIF {#CompanyNif} · Tel. {#CompanyPhone} · {#CompanySite}
en.WelcomeLabel2=This wizard will install [name/ver] on this computer.%n%nmacread reads and copies files from Mac (APFS, HFS+) and Linux (ext2/3/4, LVM) disks directly on Windows, without drivers, and can mount them as a drive letter.%n%nDeveloped by {#Company}%nVAT {#CompanyNif} · Phone {#CompanyPhone} · {#CompanySite}
pt.FinishedLabel=A instalação do [name] terminou.%n%nSuporte: {#Repo}/issues · {#Company} · {#CompanySite}
ptbr.FinishedLabel=A instalação do [name] foi concluída.%n%nSuporte: {#Repo}/issues · {#Company} · {#CompanySite}
en.FinishedLabel=Setup has finished installing [name].%n%nSupport: {#Repo}/issues · {#Company} · {#CompanySite}

[CustomMessages]
pt.TaskDokan=Instalar o driver Dokan {#DokanVersion} (necessário para montar discos como unidade; ~9 MB, será descarregado)
ptbr.TaskDokan=Instalar o driver Dokan {#DokanVersion} (necessário para montar discos como unidade; ~9 MB, será baixado)
en.TaskDokan=Install the Dokan {#DokanVersion} driver (required to mount disks as a drive letter; ~9 MB download)
pt.TaskPath=Adicionar a linha de comando (macread.exe) ao PATH
ptbr.TaskPath=Adicionar a linha de comando (macread.exe) ao PATH
en.TaskPath=Add the command-line tool (macread.exe) to PATH
pt.GroupDeps=Dependências:
ptbr.GroupDeps=Dependências:
en.GroupDeps=Dependencies:
pt.GroupOpts=Opções:
ptbr.GroupOpts=Opções:
en.GroupOpts=Options:
pt.RunApp=Abrir o macread agora
ptbr.RunApp=Abrir o macread agora
en.RunApp=Open macread now
pt.DokanFail=O driver Dokan não foi instalado (código %1). O macread funciona normalmente, mas a montagem como unidade ficará indisponível até instalar o Dokan: https://github.com/dokan-dev/dokany/releases
ptbr.DokanFail=O driver Dokan não foi instalado (código %1). O macread funciona normalmente, mas a montagem como unidade ficará indisponível até instalar o Dokan: https://github.com/dokan-dev/dokany/releases
en.DokanFail=The Dokan driver was not installed (code %1). macread still works, but mounting as a drive letter will be unavailable until Dokan is installed: https://github.com/dokan-dev/dokany/releases
pt.DokanDownloadFail=Não foi possível descarregar o driver Dokan (%1). O macread será instalado mesmo assim; a montagem como unidade ficará indisponível até instalar o Dokan: https://github.com/dokan-dev/dokany/releases
ptbr.DokanDownloadFail=Não foi possível baixar o driver Dokan (%1). O macread será instalado mesmo assim; a montagem como unidade ficará indisponível até instalar o Dokan: https://github.com/dokan-dev/dokany/releases
en.DokanDownloadFail=The Dokan driver could not be downloaded (%1). macread will be installed anyway; mounting as a drive letter will be unavailable until Dokan is installed: https://github.com/dokan-dev/dokany/releases
pt.Manual=Manual do macread
ptbr.Manual=Manual do macread
en.Manual=macread manual
pt.CmdLine=macread (linha de comando, como Administrador)
ptbr.CmdLine=macread (linha de comando, como Administrador)
en.CmdLine=macread (command line, as Administrator)
pt.Site=Site da SUDOMAKE
ptbr.Site=Site da SUDOMAKE
en.Site=SUDOMAKE website

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "addpath"; Description: "{cm:TaskPath}"; GroupDescription: "{cm:GroupOpts}"; Flags: unchecked
Name: "dokan"; Description: "{cm:TaskDokan}"; GroupDescription: "{cm:GroupDeps}"; Check: not DokanInstalled

[Files]
Source: "..\target\release\macread-gui.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\macread.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\abrir-como-admin.cmd"; DestDir: "{app}"
Source: "..\README.md"; DestDir: "{app}"
Source: "..\LICENSE"; DestDir: "{app}"
Source: "macread.ico"; DestDir: "{app}"

[Icons]
Name: "{group}\macread"; Filename: "{app}\macread-gui.exe"; IconFilename: "{app}\macread.ico"; Comment: "{#AppComments}"
Name: "{group}\{cm:CmdLine}"; Filename: "{app}\abrir-como-admin.cmd"; IconFilename: "{app}\macread.ico"
Name: "{group}\{cm:Manual}"; Filename: "{app}\README.md"
Name: "{group}\{cm:Site}"; Filename: "{#CompanySite}"
Name: "{group}\{cm:UninstallProgram,macread}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\macread"; Filename: "{app}\macread-gui.exe"; IconFilename: "{app}\macread.ico"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\SUDOMAKE\macread"; ValueType: string; ValueName: "InstallDir"; ValueData: "{app}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\SUDOMAKE\macread"; ValueType: string; ValueName: "Version"; ValueData: "{#AppVersion}"
Root: HKA; Subkey: "{code:PathRegKey}"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addpath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\macread-gui.exe"; Description: "{cm:RunApp}"; Flags: nowait postinstall skipifsilent runascurrentuser

[Code]
var
  DownloadPage: TDownloadWizardPage;

function DokanInstalled: Boolean;
begin
  Result := FileExists(ExpandConstant('{sys}\dokan2.dll')) or RegKeyExists(HKLM, 'SOFTWARE\Dokan\DokanLibrary');
end;

function PathRegKey(Param: String): String;
begin
  if IsAdminInstallMode then
    Result := 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment'
  else
    Result := 'Environment';
end;

function PathRoot: Integer;
begin
  if IsAdminInstallMode then Result := HKLM else Result := HKCU;
end;

function NeedsAddPath(Param: String): Boolean;
var
  OrigPath: String;
begin
  if not RegQueryStringValue(PathRoot, PathRegKey(''), 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Lowercase(Param) + ';', ';' + Lowercase(OrigPath) + ';') = 0;
end;

procedure RemoveFromPath(Dir: String);
var
  OrigPath, NewPath: String;
  P: Integer;
begin
  if not RegQueryStringValue(PathRoot, PathRegKey(''), 'Path', OrigPath) then exit;
  NewPath := ';' + OrigPath + ';';
  P := Pos(';' + Lowercase(Dir) + ';', Lowercase(NewPath));
  if P = 0 then exit;
  Delete(NewPath, P, Length(Dir) + 1);
  NewPath := Copy(NewPath, 2, Length(NewPath) - 2);
  RegWriteExpandStringValue(PathRoot, PathRegKey(''), 'Path', NewPath);
end;

function OnDownloadProgress(const Url, FileName: String; const Progress, ProgressMax: Int64): Boolean;
begin
  if Progress = ProgressMax then
    Log(Format('Download concluído: %s', [FileName]));
  Result := True;
end;

procedure InitializeWizard;
begin
  DownloadPage := CreateDownloadPage(SetupMessage(msgWizardPreparing), SetupMessage(msgPreparingDesc), @OnDownloadProgress);
  DownloadPage.ShowBaseNameInsteadOfUrl := True;
end;

function InstallDokan(var NeedsRestart: Boolean): Boolean;
var
  ResultCode: Integer;
  Msi, Args: String;
begin
  Result := False;
  Msi := ExpandConstant('{tmp}\Dokan_x64.msi');
  Args := '/i "' + Msi + '" /qn /norestart';
  if IsAdminInstallMode then
    Exec(ExpandConstant('{sys}\msiexec.exe'), Args, '', SW_HIDE, ewWaitUntilTerminated, ResultCode)
  else
    ShellExec('runas', ExpandConstant('{sys}\msiexec.exe'), Args, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Log(Format('msiexec Dokan: código %d', [ResultCode]));
  if ResultCode = 3010 then
  begin
    NeedsRestart := True;
    Result := True;
  end
  else if ResultCode = 0 then
    Result := True
  else
    MsgBox(FmtMessage(CustomMessage('DokanFail'), [IntToStr(ResultCode)]), mbInformation, MB_OK);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  Result := '';
  if WizardIsTaskSelected('dokan') then
  begin
    DownloadPage.Clear;
    DownloadPage.Add('{#DokanUrl}', 'Dokan_x64.msi', '{#DokanSha256}');
    DownloadPage.Show;
    try
      try
        DownloadPage.Download;
        InstallDokan(NeedsRestart);
      except
        MsgBox(FmtMessage(CustomMessage('DokanDownloadFail'), [GetExceptionMessage]), mbInformation, MB_OK);
      end;
    finally
      DownloadPage.Hide;
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;
