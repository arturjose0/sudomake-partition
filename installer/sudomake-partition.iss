; Instalador do SUDOMAKE Partition — feito em Angola por José Artur Kassala
; SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA · NIF 5002359936
; Compilar com o Inno Setup 6.1 ou mais novo:  ISCC.exe installer\sudomake-partition.iss
; O instalador não precisa de nada pré-instalado: verifica o driver Dokan 2 (montagem como
; unidade) e, se faltar, descarrega-o da página oficial e instala-o.

#define AppName "SUDOMAKE Partition"
#define AppVersion "1.3.1"
#define Author "José Artur Kassala"
#define Company "SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA"
#define CompanyNif "5002359936"
#define PhoneLocal "932693623"
#define PhoneIntl "+244 932 693 623"
#define WhatsAppUrl "https://wa.me/244932693623"
#define Email "josearturkassala0@hotmail.com"
#define CompanySite "https://sudomakes.com"
#define YouTube "https://www.youtube.com/@arturjose0"
#define GitHubUser "https://github.com/arturjose0"
#define Repo "https://github.com/arturjose0/sudomake-partition"
#define PayPalUrl "https://www.paypal.com/cgi-bin/webscr?cmd=_donations&business=josearturkassala0%40hotmail.com&item_name=SUDOMAKE+Partition&currency_code=USD"
#define DokanVersion "2.3.1.1000"
#define DokanUrl "https://github.com/dokan-dev/dokany/releases/download/v2.3.1.1000/Dokan_x64.msi"
#define DokanSha256 "69ff8cb37bfec3a75921c85ffd1c6370b50a9ec4ecef2cf3a009d488dcbf5465"
#define AppComments "Lê, copia e monta discos Mac (APFS, HFS+) e Linux (ext2/3/4, LVM) e imagens DMG no Windows. Feito em Angola."

[Setup]
AppId={{B7D3F0C2-5A6E-4C2B-9F0E-6D1A2B3C4D5E}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#Company}
AppPublisherURL={#CompanySite}
AppSupportURL={#Repo}/issues
AppUpdatesURL={#Repo}/releases
AppContact=WhatsApp {#PhoneIntl} · {#Email}
AppComments={#AppComments}
VersionInfoCompany={#Company}
VersionInfoDescription=Instalador do {#AppName}
VersionInfoProductName={#AppName}
VersionInfoVersion={#AppVersion}
DefaultDirName={autopf}\SUDOMAKE Partition
DefaultGroupName=SUDOMAKE Partition
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\sudomake-partition.ico
UninstallDisplayName={#AppName} {#AppVersion} (SUDOMAKE, Angola)
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog commandline
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\dist
OutputBaseFilename=sudomake-partition-setup-{#AppVersion}
SetupIconFile=sudomake-partition.ico
WizardSmallImageFile=wizard-small.bmp
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\LICENSE
ShowLanguageDialog=auto
ChangesEnvironment=yes

[Languages]
Name: "pt"; MessagesFile: "compiler:Languages\Portuguese.isl"; InfoBeforeFile: "sobre-pt.txt"
Name: "ptbr"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"; InfoBeforeFile: "sobre-pt.txt"
Name: "en"; MessagesFile: "compiler:Default.isl"; InfoBeforeFile: "sobre-en.txt"
Name: "fr"; MessagesFile: "compiler:Languages\French.isl"; InfoBeforeFile: "sobre-fr.txt"
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"; InfoBeforeFile: "sobre-es.txt"

[Messages]
pt.WelcomeLabel2=Este assistente vai instalar o [name/ver] neste computador.%n%nO SUDOMAKE Partition lê e copia ficheiros de discos de Mac (APFS, HFS+) e Linux (ext2/3/4, LVM) directamente no Windows, sem drivers, e pode montá-los como uma unidade.%n%nFeito em Angola por {#Author}%n{#Company} · NIF {#CompanyNif}%nWhatsApp {#PhoneIntl} · {#CompanySite}
ptbr.WelcomeLabel2=Este assistente vai instalar o [name/ver] neste computador.%n%nO SUDOMAKE Partition lê e copia arquivos de discos de Mac (APFS, HFS+) e Linux (ext2/3/4, LVM) diretamente no Windows, sem drivers, e pode montá-los como uma unidade.%n%nFeito em Angola por {#Author}%n{#Company} · NIF {#CompanyNif}%nWhatsApp {#PhoneIntl} · {#CompanySite}
en.WelcomeLabel2=This wizard will install [name/ver] on this computer.%n%nSUDOMAKE Partition reads and copies files from Mac (APFS, HFS+) and Linux (ext2/3/4, LVM) disks directly on Windows, without drivers, and can mount them as a drive letter.%n%nMade in Angola by {#Author}%n{#Company} · VAT {#CompanyNif}%nWhatsApp {#PhoneIntl} · {#CompanySite}
fr.WelcomeLabel2=Cet assistant va installer [name/ver] sur cet ordinateur.%n%nSUDOMAKE Partition lit et copie les fichiers des disques Mac (APFS, HFS+) et Linux (ext2/3/4, LVM) directement sous Windows, sans pilote, et peut les monter comme un lecteur.%n%nFabriqué en Angola par {#Author}%n{#Company} · NIF {#CompanyNif}%nWhatsApp {#PhoneIntl} · {#CompanySite}
es.WelcomeLabel2=Este asistente instalará [name/ver] en este equipo.%n%nSUDOMAKE Partition lee y copia archivos de discos Mac (APFS, HFS+) y Linux (ext2/3/4, LVM) directamente en Windows, sin controladores, y puede montarlos como una unidad.%n%nHecho en Angola por {#Author}%n{#Company} · NIF {#CompanyNif}%nWhatsApp {#PhoneIntl} · {#CompanySite}
pt.FinishedLabel=A instalação do [name] terminou.%n%nSe o programa o ajudar, considere uma doação (não obrigatória): PayPal {#Email} · Transferência Express {#PhoneLocal}. Depois envie o comprovativo pelo WhatsApp ({#PhoneIntl}) ou por e-mail. Siga o canal: {#YouTube}%n%nSuporte: {#Repo}/issues
ptbr.FinishedLabel=A instalação do [name] foi concluída.%n%nSe o programa ajudar você, considere uma doação (não obrigatória): PayPal {#Email} · Transferência Express {#PhoneLocal}. Depois envie o comprovante pelo WhatsApp ({#PhoneIntl}) ou por e-mail. Siga o canal: {#YouTube}%n%nSuporte: {#Repo}/issues
en.FinishedLabel=Setup has finished installing [name].%n%nIf the program helps you, please consider a donation (not required): PayPal {#Email} · Express transfer (Angola) {#PhoneLocal}. Then send the receipt via WhatsApp ({#PhoneIntl}) or e-mail. Follow the channel: {#YouTube}%n%nSupport: {#Repo}/issues
fr.FinishedLabel=L'installation de [name] est terminée.%n%nSi le programme vous aide, pensez à faire un don (facultatif) : PayPal {#Email} · Transfert Express (Angola) {#PhoneLocal}. Envoyez ensuite le reçu par WhatsApp ({#PhoneIntl}) ou par e-mail. Suivez la chaîne : {#YouTube}%n%nAssistance : {#Repo}/issues
es.FinishedLabel=La instalación de [name] ha terminado.%n%nSi el programa le ayuda, considere una donación (no obligatoria): PayPal {#Email} · Transferencia Express (Angola) {#PhoneLocal}. Luego envíe el comprobante por WhatsApp ({#PhoneIntl}) o por correo. Siga el canal: {#YouTube}%n%nSoporte: {#Repo}/issues

[CustomMessages]
pt.TaskDokan=Instalar o driver Dokan {#DokanVersion} (necessário para montar discos como unidade; ~9 MB, será descarregado)
ptbr.TaskDokan=Instalar o driver Dokan {#DokanVersion} (necessário para montar discos como unidade; ~9 MB, será baixado)
en.TaskDokan=Install the Dokan {#DokanVersion} driver (required to mount disks as a drive letter; ~9 MB download)
fr.TaskDokan=Installer le pilote Dokan {#DokanVersion} (nécessaire pour monter les disques comme lecteur ; ~9 Mo à télécharger)
es.TaskDokan=Instalar el controlador Dokan {#DokanVersion} (necesario para montar discos como unidad; ~9 MB, se descargará)
pt.TaskPath=Adicionar a linha de comando (sudomake-partition.exe) ao PATH
ptbr.TaskPath=Adicionar a linha de comando (sudomake-partition.exe) ao PATH
en.TaskPath=Add the command-line tool (sudomake-partition.exe) to PATH
fr.TaskPath=Ajouter l'outil en ligne de commande (sudomake-partition.exe) au PATH
es.TaskPath=Añadir la herramienta de línea de comandos (sudomake-partition.exe) al PATH
pt.GroupDeps=Dependências:
ptbr.GroupDeps=Dependências:
en.GroupDeps=Dependencies:
fr.GroupDeps=Dépendances :
es.GroupDeps=Dependencias:
pt.GroupOpts=Opções:
ptbr.GroupOpts=Opções:
en.GroupOpts=Options:
fr.GroupOpts=Options :
es.GroupOpts=Opciones:
pt.RunApp=Abrir o SUDOMAKE Partition agora
ptbr.RunApp=Abrir o SUDOMAKE Partition agora
en.RunApp=Open SUDOMAKE Partition now
fr.RunApp=Ouvrir SUDOMAKE Partition maintenant
es.RunApp=Abrir SUDOMAKE Partition ahora
pt.OpenYouTube=Seguir o canal no YouTube (@arturjose0)
ptbr.OpenYouTube=Seguir o canal no YouTube (@arturjose0)
en.OpenYouTube=Follow the YouTube channel (@arturjose0)
fr.OpenYouTube=Suivre la chaîne YouTube (@arturjose0)
es.OpenYouTube=Seguir el canal de YouTube (@arturjose0)
pt.DokanFail=O driver Dokan não foi instalado (código %1). O SUDOMAKE Partition funciona normalmente, mas a montagem como unidade ficará indisponível até instalar o Dokan: https://github.com/dokan-dev/dokany/releases
ptbr.DokanFail=O driver Dokan não foi instalado (código %1). O SUDOMAKE Partition funciona normalmente, mas a montagem como unidade ficará indisponível até instalar o Dokan: https://github.com/dokan-dev/dokany/releases
en.DokanFail=The Dokan driver was not installed (code %1). SUDOMAKE Partition still works, but mounting as a drive letter will be unavailable until Dokan is installed: https://github.com/dokan-dev/dokany/releases
fr.DokanFail=Le pilote Dokan n'a pas été installé (code %1). SUDOMAKE Partition fonctionne quand même, mais le montage comme lecteur restera indisponible tant que Dokan n'est pas installé : https://github.com/dokan-dev/dokany/releases
es.DokanFail=El controlador Dokan no se instaló (código %1). SUDOMAKE Partition funciona igualmente, pero el montaje como unidad no estará disponible hasta instalar Dokan: https://github.com/dokan-dev/dokany/releases
pt.DokanDownloadFail=Não foi possível descarregar o driver Dokan (%1). O programa será instalado mesmo assim; a montagem como unidade ficará indisponível até instalar o Dokan: https://github.com/dokan-dev/dokany/releases
ptbr.DokanDownloadFail=Não foi possível baixar o driver Dokan (%1). O programa será instalado mesmo assim; a montagem como unidade ficará indisponível até instalar o Dokan: https://github.com/dokan-dev/dokany/releases
en.DokanDownloadFail=The Dokan driver could not be downloaded (%1). The program will be installed anyway; mounting as a drive letter will be unavailable until Dokan is installed: https://github.com/dokan-dev/dokany/releases
fr.DokanDownloadFail=Impossible de télécharger le pilote Dokan (%1). Le programme sera installé quand même ; le montage comme lecteur restera indisponible tant que Dokan n'est pas installé : https://github.com/dokan-dev/dokany/releases
es.DokanDownloadFail=No se pudo descargar el controlador Dokan (%1). El programa se instalará de todos modos; el montaje como unidad no estará disponible hasta instalar Dokan: https://github.com/dokan-dev/dokany/releases
pt.Manual=Manual do SUDOMAKE Partition
ptbr.Manual=Manual do SUDOMAKE Partition
en.Manual=SUDOMAKE Partition manual
fr.Manual=Manuel de SUDOMAKE Partition
es.Manual=Manual de SUDOMAKE Partition
pt.CmdLine=SUDOMAKE Partition (linha de comando, como Administrador)
ptbr.CmdLine=SUDOMAKE Partition (linha de comando, como Administrador)
en.CmdLine=SUDOMAKE Partition (command line, as Administrator)
fr.CmdLine=SUDOMAKE Partition (ligne de commande, en Administrateur)
es.CmdLine=SUDOMAKE Partition (línea de comandos, como Administrador)
pt.Site=Site da SUDOMAKE
ptbr.Site=Site da SUDOMAKE
en.Site=SUDOMAKE website
fr.Site=Site de SUDOMAKE
es.Site=Sitio web de SUDOMAKE
pt.Donate=Apoiar o desenvolvedor (doação)
ptbr.Donate=Apoiar o desenvolvedor (doação)
en.Donate=Support the developer (donation)
fr.Donate=Soutenir le développeur (don)
es.Donate=Apoyar al desarrollador (donación)
pt.DonatePageTitle=Apoie o desenvolvedor — Feito em Angola
ptbr.DonatePageTitle=Apoie o desenvolvedor — Feito em Angola
en.DonatePageTitle=Support the developer — Made in Angola
fr.DonatePageTitle=Soutenez le développeur — Fabriqué en Angola
es.DonatePageTitle=Apoye al desarrollador — Hecho en Angola
pt.DonatePageDesc=O SUDOMAKE Partition é gratuito e de código aberto. A doação não é obrigatória.
ptbr.DonatePageDesc=O SUDOMAKE Partition é gratuito e de código aberto. A doação não é obrigatória.
en.DonatePageDesc=SUDOMAKE Partition is free and open source. Donating is not required.
fr.DonatePageDesc=SUDOMAKE Partition est gratuit et open source. Le don est facultatif.
es.DonatePageDesc=SUDOMAKE Partition es gratuito y de código abierto. La donación no es obligatoria.
pt.DonateIntro=Este programa foi desenvolvido em Angola por {#Author} ({#Company}, NIF {#CompanyNif}). Se ele o ajudar a recuperar os seus ficheiros, considere uma doação de qualquer valor:
ptbr.DonateIntro=Este programa foi desenvolvido em Angola por {#Author} ({#Company}, NIF {#CompanyNif}). Se ele ajudar você a recuperar seus arquivos, considere uma doação de qualquer valor:
en.DonateIntro=This program was developed in Angola by {#Author} ({#Company}, VAT {#CompanyNif}). If it helps you recover your files, please consider a donation of any amount:
fr.DonateIntro=Ce programme a été développé en Angola par {#Author} ({#Company}, NIF {#CompanyNif}). S'il vous aide à récupérer vos fichiers, pensez à faire un don du montant de votre choix :
es.DonateIntro=Este programa fue desarrollado en Angola por {#Author} ({#Company}, NIF {#CompanyNif}). Si le ayuda a recuperar sus archivos, considere una donación de cualquier importe:
pt.DonatePayPal=PayPal: {#Email}   (clique para abrir)
ptbr.DonatePayPal=PayPal: {#Email}   (clique para abrir)
en.DonatePayPal=PayPal: {#Email}   (click to open)
fr.DonatePayPal=PayPal : {#Email}   (cliquer pour ouvrir)
es.DonatePayPal=PayPal: {#Email}   (clic para abrir)
pt.DonateExpress=Transferência Express (Angola): {#PhoneLocal}
ptbr.DonateExpress=Transferência Express (Angola): {#PhoneLocal}
en.DonateExpress=Express transfer (Angola): {#PhoneLocal}
fr.DonateExpress=Transfert Express (Angola) : {#PhoneLocal}
es.DonateExpress=Transferencia Express (Angola): {#PhoneLocal}
pt.DonateAfter=Depois de doar, envie o comprovativo pelo WhatsApp ou por e-mail para podermos agradecer:
ptbr.DonateAfter=Depois de doar, envie o comprovante pelo WhatsApp ou por e-mail para podermos agradecer:
en.DonateAfter=After donating, send the receipt via WhatsApp or e-mail so we can thank you:
fr.DonateAfter=Après votre don, envoyez le reçu par WhatsApp ou par e-mail pour que nous puissions vous remercier :
es.DonateAfter=Después de donar, envíe el comprobante por WhatsApp o por correo para poder agradecerle:
pt.LinkWhatsApp=WhatsApp: {#PhoneIntl}
ptbr.LinkWhatsApp=WhatsApp: {#PhoneIntl}
en.LinkWhatsApp=WhatsApp: {#PhoneIntl}
fr.LinkWhatsApp=WhatsApp : {#PhoneIntl}
es.LinkWhatsApp=WhatsApp: {#PhoneIntl}
pt.LinkEmail=E-mail: {#Email}
ptbr.LinkEmail=E-mail: {#Email}
en.LinkEmail=E-mail: {#Email}
fr.LinkEmail=E-mail : {#Email}
es.LinkEmail=Correo: {#Email}
pt.DonateFollow=Siga também o canal no YouTube e o GitHub para acompanhar novas versões:
ptbr.DonateFollow=Siga também o canal no YouTube e o GitHub para acompanhar novas versões:
en.DonateFollow=Also follow the YouTube channel and GitHub for new versions:
fr.DonateFollow=Suivez aussi la chaîne YouTube et le GitHub pour les nouvelles versions :
es.DonateFollow=Siga también el canal de YouTube y GitHub para conocer nuevas versiones:
pt.LinkYouTube=YouTube: {#YouTube}
ptbr.LinkYouTube=YouTube: {#YouTube}
en.LinkYouTube=YouTube: {#YouTube}
fr.LinkYouTube=YouTube : {#YouTube}
es.LinkYouTube=YouTube: {#YouTube}
pt.LinkGitHub=GitHub: {#GitHubUser}   ·   Site: {#CompanySite}
ptbr.LinkGitHub=GitHub: {#GitHubUser}   ·   Site: {#CompanySite}
en.LinkGitHub=GitHub: {#GitHubUser}   ·   Website: {#CompanySite}
fr.LinkGitHub=GitHub : {#GitHubUser}   ·   Site : {#CompanySite}
es.LinkGitHub=GitHub: {#GitHubUser}   ·   Sitio: {#CompanySite}

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "addpath"; Description: "{cm:TaskPath}"; GroupDescription: "{cm:GroupOpts}"; Flags: unchecked
Name: "dokan"; Description: "{cm:TaskDokan}"; GroupDescription: "{cm:GroupDeps}"; Check: not DokanInstalled

[InstallDelete]
; versões anteriores (macread 1.x) instaladas na mesma pasta
Type: files; Name: "{app}\macread-gui.exe"
Type: files; Name: "{app}\macread.exe"
Type: files; Name: "{app}\macread.ico"
Type: filesandordirs; Name: "{group}\macread"
Type: files; Name: "{autodesktop}\macread.lnk"

[Files]
Source: "..\target\release\SudomakePartition.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\sudomake-partition.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\abrir-como-admin.cmd"; DestDir: "{app}"
Source: "..\README.md"; DestDir: "{app}"
Source: "..\LICENSE"; DestDir: "{app}"
Source: "sudomake-partition.ico"; DestDir: "{app}"

[Icons]
Name: "{group}\SUDOMAKE Partition"; Filename: "{app}\SudomakePartition.exe"; IconFilename: "{app}\sudomake-partition.ico"; Comment: "{#AppComments}"
Name: "{group}\{cm:CmdLine}"; Filename: "{app}\abrir-como-admin.cmd"; IconFilename: "{app}\sudomake-partition.ico"
Name: "{group}\{cm:Manual}"; Filename: "{app}\README.md"
Name: "{group}\{cm:Site}"; Filename: "{#CompanySite}"
Name: "{group}\{cm:Donate}"; Filename: "{#PayPalUrl}"
Name: "{group}\{cm:UninstallProgram,SUDOMAKE Partition}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\SUDOMAKE Partition"; Filename: "{app}\SudomakePartition.exe"; IconFilename: "{app}\sudomake-partition.ico"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\SUDOMAKE\Partition"; ValueType: string; ValueName: "InstallDir"; ValueData: "{app}"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\SUDOMAKE\Partition"; ValueType: string; ValueName: "Version"; ValueData: "{#AppVersion}"
Root: HKA; Subkey: "{code:PathRegKey}"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addpath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\SudomakePartition.exe"; Description: "{cm:RunApp}"; Flags: nowait postinstall skipifsilent runascurrentuser
Filename: "{#YouTube}"; Description: "{cm:OpenYouTube}"; Flags: shellexec nowait postinstall skipifsilent unchecked

[Code]
var
  DownloadPage: TDownloadWizardPage;
  DonatePage: TWizardPage;

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

{ ---- página de doação com ligações clicáveis ---- }

procedure OpenLink(Sender: TObject);
var
  ErrorCode: Integer;
begin
  ShellExec('open', TNewStaticText(Sender).Hint, '', '', SW_SHOWNORMAL, ewNoWait, ErrorCode);
end;

function AddText(Page: TWizardPage; Top: Integer; const Text: String; Bold: Boolean): TNewStaticText;
begin
  Result := TNewStaticText.Create(Page);
  Result.Parent := Page.Surface;
  Result.Left := 0;
  Result.Top := Top;
  Result.Width := Page.SurfaceWidth;
  Result.WordWrap := True;
  Result.AutoSize := True;
  Result.Caption := Text;
  if Bold then Result.Font.Style := [fsBold];
end;

function AddLink(Page: TWizardPage; Top: Integer; const Text, Url: String): TNewStaticText;
begin
  Result := AddText(Page, Top, Text, False);
  Result.Font.Color := clBlue;
  Result.Font.Style := [fsUnderline];
  Result.Cursor := crHand;
  Result.Hint := Url;
  Result.OnClick := @OpenLink;
end;

procedure CreateDonatePage;
var
  Y: Integer;
  L: TNewStaticText;
begin
  DonatePage := CreateCustomPage(wpInfoBefore, CustomMessage('DonatePageTitle'), CustomMessage('DonatePageDesc'));
  Y := 0;
  L := AddText(DonatePage, Y, CustomMessage('DonateIntro'), False);
  Y := L.Top + L.Height + ScaleY(10);
  L := AddLink(DonatePage, Y, CustomMessage('DonatePayPal'), '{#PayPalUrl}');
  Y := L.Top + L.Height + ScaleY(4);
  L := AddText(DonatePage, Y, CustomMessage('DonateExpress'), True);
  Y := L.Top + L.Height + ScaleY(12);
  L := AddText(DonatePage, Y, CustomMessage('DonateAfter'), False);
  Y := L.Top + L.Height + ScaleY(4);
  L := AddLink(DonatePage, Y, CustomMessage('LinkWhatsApp'), '{#WhatsAppUrl}');
  Y := L.Top + L.Height + ScaleY(4);
  L := AddLink(DonatePage, Y, CustomMessage('LinkEmail'), 'mailto:{#Email}?subject=SUDOMAKE%20Partition');
  Y := L.Top + L.Height + ScaleY(12);
  L := AddText(DonatePage, Y, CustomMessage('DonateFollow'), False);
  Y := L.Top + L.Height + ScaleY(4);
  L := AddLink(DonatePage, Y, CustomMessage('LinkYouTube'), '{#YouTube}');
  Y := L.Top + L.Height + ScaleY(4);
  L := AddLink(DonatePage, Y, CustomMessage('LinkGitHub'), '{#GitHubUser}');
end;

procedure InitializeWizard;
begin
  DownloadPage := CreateDownloadPage(SetupMessage(msgWizardPreparing), SetupMessage(msgPreparingDesc), @OnDownloadProgress);
  DownloadPage.ShowBaseNameInsteadOfUrl := True;
  CreateDonatePage;
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
