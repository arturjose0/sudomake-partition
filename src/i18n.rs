//! Textos da interface em português (Angola), inglês, francês e espanhol.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Pt,
    En,
    Fr,
    Es,
}

impl Lang {
    pub const ALL: [Lang; 4] = [Lang::Pt, Lang::En, Lang::Fr, Lang::Es];

    pub fn code(self) -> &'static str {
        match self {
            Lang::Pt => "pt",
            Lang::En => "en",
            Lang::Fr => "fr",
            Lang::Es => "es",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Lang::Pt => "Português (Angola)",
            Lang::En => "English (US)",
            Lang::Fr => "Français",
            Lang::Es => "Español",
        }
    }

    pub fn from_code(s: &str) -> Option<Lang> {
        match s.trim().to_lowercase().as_str() {
            "pt" | "pt-ao" | "pt-br" | "pt-pt" => Some(Lang::Pt),
            "en" | "en-us" => Some(Lang::En),
            "fr" => Some(Lang::Fr),
            "es" => Some(Lang::Es),
            _ => None,
        }
    }

    /// Idioma a partir do identificador de idioma do Windows (LANGID).
    pub fn from_langid(id: u16) -> Lang {
        match id & 0x3FF {
            0x16 => Lang::Pt,
            0x0C => Lang::Fr,
            0x0A => Lang::Es,
            _ => Lang::En,
        }
    }
}

/// Dados de contacto e marketing (iguais em todos os idiomas).
pub const APP_NAME: &str = "SUDOMAKE Partition";
pub const COMPANY: &str = "SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA";
pub const COMPANY_NIF: &str = "5002359936";
pub const PHONE_DISPLAY: &str = "+244 932 693 623";
pub const PHONE_LOCAL: &str = "932693623";
pub const WHATSAPP_URL: &str = "https://wa.me/244932693623?text=Ol%C3%A1%21%20Uso%20o%20SUDOMAKE%20Partition.";
pub const EMAIL: &str = "josearturkassala0@hotmail.com";
pub const EMAIL_URL: &str = "mailto:josearturkassala0@hotmail.com?subject=SUDOMAKE%20Partition";
pub const SITE: &str = "https://sudomakes.com";
pub const YOUTUBE: &str = "https://www.youtube.com/@arturjose0";
pub const GITHUB_USER: &str = "https://github.com/arturjose0";
pub const REPO: &str = "https://github.com/arturjose0/sudomake-partition";
pub const PAYPAL_URL: &str = "https://www.paypal.com/cgi-bin/webscr?cmd=_donations&business=josearturkassala0%40hotmail.com&item_name=SUDOMAKE+Partition&currency_code=USD";
pub const AUTHOR: &str = "José Artur Kassala";

macro_rules! table {
    ($( $key:ident => [$pt:expr, $en:expr, $fr:expr, $es:expr] ),* $(,)?) => {
        /// Devolve o texto `key` no idioma pedido.
        pub fn tr(lang: Lang, key: &str) -> &'static str {
            match key {
                $( stringify!($key) => match lang { Lang::Pt => $pt, Lang::En => $en, Lang::Fr => $fr, Lang::Es => $es }, )*
                _ => key_missing(key),
            }
        }
    };
}

fn key_missing(key: &str) -> &'static str {
    // chave desconhecida: devolve a própria chave (vazada de propósito; só ocorre por erro de programação)
    Box::leak(key.to_string().into_boxed_str())
}

table! {
    // ---- barra de ferramentas
    btn_home => ["Discos", "Disks", "Disques", "Discos"],
    btn_image => ["Abrir imagem/DMG...", "Open image/DMG...", "Ouvrir une image/DMG...", "Abrir imagen/DMG..."],
    btn_up => ["▲ Acima", "▲ Up", "▲ Parent", "▲ Arriba"],
    btn_copy_sel => ["Copiar selecionados...", "Copy selected...", "Copier la sélection...", "Copiar seleccionados..."],
    btn_copy_all => ["Copiar pasta atual...", "Copy current folder...", "Copier le dossier actuel...", "Copiar carpeta actual..."],
    btn_verify => ["Verificar leitura", "Verify read", "Vérifier la lecture", "Verificar lectura"],
    btn_mount => ["Montar como unidade", "Mount as drive", "Monter comme lecteur", "Montar como unidad"],
    btn_unmount => ["Desmontar {0}:", "Unmount {0}:", "Démonter {0}:", "Desmontar {0}:"],
    btn_cancel => ["Cancelar", "Cancel", "Annuler", "Cancelar"],
    btn_copy_disk => ["Copiar disco completo...", "Copy entire disk...", "Copier le disque entier...", "Copiar disco completo..."],
    copy_disk_title => ["Copiar disco completo", "Copy entire disk", "Copier le disque entier", "Copiar disco completo"],
    copy_disk_confirm => [
        "Copiar todo o conteúdo de {0} ({1} usados) para a pasta:\n{2}\n\nEspaço livre no destino: {3}.\n\nContinuar?",
        "Copy the entire contents of {0} ({1} used) to the folder:\n{2}\n\nFree space at destination: {3}.\n\nContinue?",
        "Copier tout le contenu de {0} ({1} utilisés) vers le dossier :\n{2}\n\nEspace libre à destination : {3}.\n\nContinuer ?",
        "¿Copiar todo el contenido de {0} ({1} usados) a la carpeta:\n{2}\n\nEspacio libre en el destino: {3}.\n\n¿Continuar?"
    ],
    space_insufficient => [
        "Espaço insuficiente no destino.\n\nNecessário: {0}\nLivre em {1}: {2}\n\nEscolha outro disco ou liberte espaço.",
        "Not enough space at the destination.\n\nNeeded: {0}\nFree on {1}: {2}\n\nChoose another disk or free some space.",
        "Espace insuffisant à destination.\n\nNécessaire : {0}\nLibre sur {1} : {2}\n\nChoisissez un autre disque ou libérez de l'espace.",
        "Espacio insuficiente en el destino.\n\nNecesario: {0}\nLibre en {1}: {2}\n\nElija otro disco o libere espacio."
    ],
    space_unknown => [
        "Não foi possível calcular o espaço necessário. A cópia vai continuar e pára se o destino encher. Continuar?",
        "The required space could not be calculated. The copy will proceed and stop if the destination fills up. Continue?",
        "Impossible de calculer l'espace nécessaire. La copie continuera et s'arrêtera si la destination est pleine. Continuer ?",
        "No se pudo calcular el espacio necesario. La copia continuará y se detendrá si el destino se llena. ¿Continuar?"
    ],
    disks_updated => ["Discos actualizados.", "Disks updated.", "Disques mis à jour.", "Discos actualizados."],
    mount_lost => ["A unidade {0}: foi desmontada porque o disco foi removido.", "Drive {0}: was unmounted because the disk was removed.", "Le lecteur {0}: a été démonté car le disque a été retiré.", "La unidad {0}: se desmontó porque el disco fue retirado."],
    mounted_short => ["montado em {0}:", "mounted as {0}:", "monté en {0}:", "montado en {0}:"],
    btn_about => ["Sobre", "About", "À propos", "Acerca de"],
    btn_donate => ["Doar ❤", "Donate ❤", "Faire un don ❤", "Donar ❤"],
    lbl_lang => ["Idioma:", "Language:", "Langue :", "Idioma:"],
    lbl_theme => ["Tema:", "Theme:", "Thème :", "Tema:"],
    theme_system => ["Sistema", "System", "Système", "Sistema"],
    theme_light => ["Claro", "Light", "Clair", "Claro"],
    theme_dark => ["Escuro", "Dark", "Sombre", "Oscuro"],
    restart_lang => ["O idioma será aplicado por completo na próxima vez que abrir o programa.", "The language will be fully applied the next time you open the program.", "La langue sera entièrement appliquée au prochain démarrage du programme.", "El idioma se aplicará por completo la próxima vez que abra el programa."],

    // ---- ecrã inicial
    welcome_title => ["Bem-vindo ao SUDOMAKE Partition", "Welcome to SUDOMAKE Partition", "Bienvenue dans SUDOMAKE Partition", "Bienvenido a SUDOMAKE Partition"],
    welcome_text => [
        "Abaixo estão os discos ligados a este computador e as imagens abertas, com o sistema encontrado em cada partição (Windows, Linux, macOS, dados...).\r\n\r\nClique num disco ou volume e escolha: Abrir no programa (navegar e copiar ficheiros) ou Montar como unidade (aparece no Explorador como uma letra de disco, somente leitura). Nada é gravado no disco de origem.",
        "Below are the disks connected to this computer and the opened images, with the system found on each partition (Windows, Linux, macOS, data...).\r\n\r\nClick a disk or volume and choose: Open in the program (browse and copy files) or Mount as drive (it appears in Explorer as a drive letter, read-only). Nothing is written to the source disk.",
        "Ci-dessous se trouvent les disques connectés à cet ordinateur et les images ouvertes, avec le système trouvé sur chaque partition (Windows, Linux, macOS, données...).\r\n\r\nCliquez sur un disque ou un volume et choisissez : Ouvrir dans le programme (parcourir et copier des fichiers) ou Monter comme lecteur (il apparaît dans l'Explorateur avec une lettre, en lecture seule). Rien n'est écrit sur le disque d'origine.",
        "Abajo están los discos conectados a este equipo y las imágenes abiertas, con el sistema encontrado en cada partición (Windows, Linux, macOS, datos...).\r\n\r\nHaga clic en un disco o volumen y elija: Abrir en el programa (navegar y copiar archivos) o Montar como unidad (aparece en el Explorador como una letra de unidad, solo lectura). No se escribe nada en el disco de origen."
    ],
    searching => ["A procurar discos e a identificar os sistemas instalados...", "Looking for disks and identifying the installed systems...", "Recherche des disques et identification des systèmes installés...", "Buscando discos e identificando los sistemas instalados..."],
    select_hint => ["Selecione um disco ou volume.", "Select a disk or volume.", "Sélectionnez un disque ou un volume.", "Seleccione un disco o volumen."],
    choose_action => ["O que deseja fazer com este volume?", "What do you want to do with this volume?", "Que voulez-vous faire avec ce volume ?", "¿Qué desea hacer con este volumen?"],
    btn_open_here => ["Abrir no programa", "Open in the program", "Ouvrir dans le programme", "Abrir en el programa"],
    btn_mount_here => ["Montar como unidade", "Mount as drive", "Monter comme lecteur", "Montar como unidad"],
    disk => ["Disco", "Disk", "Disque", "Disco"],
    image => ["Imagem", "Image", "Image", "Imagen"],
    partition => ["Partição", "Partition", "Partition", "Partición"],
    volume => ["Volume", "Volume", "Volume", "Volumen"],
    no_disks => ["Nenhum disco físico encontrado.", "No physical disk found.", "Aucun disque physique trouvé.", "No se encontró ningún disco físico."],
    no_permission => ["sem permissão de Administrador", "no Administrator permission", "sans permission Administrateur", "sin permiso de Administrador"],
    no_permission_long => ["Para ler discos físicos o programa precisa de ser executado como Administrador. Feche e abra de novo aceitando o pedido de permissão.", "To read physical disks the program must run as Administrator. Close it and open it again accepting the permission request.", "Pour lire les disques physiques, le programme doit être exécuté en tant qu'Administrateur. Fermez-le et rouvrez-le en acceptant la demande.", "Para leer discos físicos el programa debe ejecutarse como Administrador. Ciérrelo y ábralo de nuevo aceptando la solicitud de permiso."],
    free_of => ["{0} livres de {1}", "{0} free of {1}", "{0} libres sur {1}", "{0} libres de {1}"],
    size_only => ["{0}", "{0}", "{0}", "{0}"],
    not_readable => ["Esta partição não pode ser aberta pelo programa.", "This partition cannot be opened by the program.", "Cette partition ne peut pas être ouverte par le programme.", "Esta partición no puede abrirse con el programa."],
    windows_reads => ["O próprio Windows já a lê, se estiver íntegra.", "Windows itself reads it, if it is intact.", "Windows la lit lui-même, si elle est intacte.", "El propio Windows la lee, si está íntegra."],
    encrypted_volume => ["criptografado (FileVault): precisa da senha, não suportado", "encrypted (FileVault): needs the password, not supported", "chiffré (FileVault) : mot de passe requis, non pris en charge", "cifrado (FileVault): necesita la contraseña, no compatible"],
    cannot_open => ["não foi possível abrir", "could not open", "impossible d'ouvrir", "no se pudo abrir"],
    container_apfs => ["container APFS com {0} volumes", "APFS container with {0} volumes", "conteneur APFS avec {0} volumes", "contenedor APFS con {0} volúmenes"],
    systems_found => ["Sistemas encontrados: {0}", "Systems found: {0}", "Systèmes trouvés : {0}", "Sistemas encontrados: {0}"],
    none_recognized => ["nenhum reconhecido", "none recognized", "aucun reconnu", "ninguno reconocido"],
    fs_label => ["Sistema de ficheiros", "File system", "Système de fichiers", "Sistema de archivos"],
    size_label => ["Tamanho", "Size", "Taille", "Tamaño"],
    type_label => ["Tipo", "Type", "Type", "Tipo"],
    role_label => ["Função", "Role", "Rôle", "Función"],
    files_folders => ["{0} ficheiros, {1} pastas", "{0} files, {1} folders", "{0} fichiers, {1} dossiers", "{0} archivos, {1} carpetas"],
    mounted_here => ["Este volume está montado como unidade do Windows.", "This volume is mounted as a Windows drive.", "Ce volume est monté comme lecteur Windows.", "Este volumen está montado como unidad de Windows."],

    // ---- sistemas detectados
    os_linux => ["{0} (sistema Linux)", "{0} (Linux system)", "{0} (système Linux)", "{0} (sistema Linux)"],
    os_macos => ["{0} {1} (sistema)", "{0} {1} (system)", "{0} {1} (système)", "{0} {1} (sistema)"],
    os_mac_data => ["macOS: dados do utilizador", "macOS: user data", "macOS : données utilisateur", "macOS: datos del usuario"],
    os_linux_home => ["Linux: pasta /home", "Linux: /home folder", "Linux : dossier /home", "Linux: carpeta /home"],
    os_linux_generic => ["Linux (sistema)", "Linux (system)", "Linux (système)", "Linux (sistema)"],
    os_mac_generic => ["macOS (versão não identificada)", "macOS (version not identified)", "macOS (version non identifiée)", "macOS (versión no identificada)"],
    os_timemachine => ["Backup do Time Machine", "Time Machine backup", "Sauvegarde Time Machine", "Copia de seguridad de Time Machine"],
    os_data => ["disco de dados ({0} itens na raiz)", "data disk ({0} items in root)", "disque de données ({0} éléments à la racine)", "disco de datos ({0} elementos en la raíz)"],
    os_empty => ["vazio", "empty", "vide", "vacío"],
    part_windows => ["Windows (NTFS)", "Windows (NTFS)", "Windows (NTFS)", "Windows (NTFS)"],
    part_windows_recovery => ["Windows: recuperação/sistema", "Windows: recovery/system", "Windows : récupération/système", "Windows: recuperación/sistema"],
    part_efi => ["EFI (inicialização)", "EFI (boot)", "EFI (démarrage)", "EFI (arranque)"],
    part_fat => ["dados (FAT)", "data (FAT)", "données (FAT)", "datos (FAT)"],
    part_exfat => ["dados (exFAT)", "data (exFAT)", "données (exFAT)", "datos (exFAT)"],
    part_swap => ["Linux swap", "Linux swap", "Linux swap", "Linux swap"],
    part_luks => ["Linux criptografado (LUKS)", "encrypted Linux (LUKS)", "Linux chiffré (LUKS)", "Linux cifrado (LUKS)"],
    part_lvm => ["LVM (os volumes lógicos aparecem em separado)", "LVM (logical volumes are listed separately)", "LVM (les volumes logiques sont listés séparément)", "LVM (los volúmenes lógicos aparecen por separado)"],
    part_xfs => ["Linux (XFS, ainda não legível)", "Linux (XFS, not readable yet)", "Linux (XFS, pas encore lisible)", "Linux (XFS, aún no legible)"],
    part_btrfs => ["Linux (Btrfs, ainda não legível)", "Linux (Btrfs, not readable yet)", "Linux (Btrfs, pas encore lisible)", "Linux (Btrfs, aún no legible)"],
    part_f2fs => ["Linux (F2FS, ainda não legível)", "Linux (F2FS, not readable yet)", "Linux (F2FS, pas encore lisible)", "Linux (F2FS, aún no legible)"],
    part_corestorage => ["macOS Core Storage (FileVault antigo/Fusion, não legível)", "macOS Core Storage (old FileVault/Fusion, not readable)", "macOS Core Storage (ancien FileVault/Fusion, non lisible)", "macOS Core Storage (FileVault antiguo/Fusion, no legible)"],
    part_hfs_classic => ["HFS clássico (não legível)", "classic HFS (not readable)", "HFS classique (non lisible)", "HFS clásico (no legible)"],
    part_reserved => ["reservado (Windows)", "reserved (Windows)", "réservé (Windows)", "reservado (Windows)"],
    part_empty => ["vazio", "empty", "vide", "vacío"],
    fs_unknown => ["desconhecido", "unknown", "inconnu", "desconocido"],
    fs_hfs_classic => ["HFS clássico", "classic HFS", "HFS classique", "HFS clásico"],
    fs_luks => ["LUKS (cifrado)", "LUKS (encrypted)", "LUKS (chiffré)", "LUKS (cifrado)"],
    part_boot => ["inicialização", "boot", "démarrage", "arranque"],
    part_recovery => ["recuperação do Windows", "Windows recovery", "récupération Windows", "recuperación de Windows"],
    part_unknown => ["desconhecido", "unknown", "inconnu", "desconocido"],

    // ---- navegação
    col_name => ["Nome", "Name", "Nom", "Nombre"],
    col_size => ["Tamanho", "Size", "Taille", "Tamaño"],
    col_modified => ["Modificado", "Modified", "Modifié", "Modificado"],
    col_type => ["Tipo", "Type", "Type", "Tipo"],
    kind_folder => ["Pasta", "Folder", "Dossier", "Carpeta"],
    kind_file => ["Ficheiro", "File", "Fichier", "Archivo"],
    kind_file_compressed => ["Ficheiro (comprimido)", "File (compressed)", "Fichier (compressé)", "Archivo (comprimido)"],
    kind_link => ["Link → {0}", "Link → {0}", "Lien → {0}", "Enlace → {0}"],
    kind_special => ["Especial", "Special", "Spécial", "Especial"],
    folder_stats => ["{0} pastas, {1} ficheiros ({2})", "{0} folders, {1} files ({2})", "{0} dossiers, {1} fichiers ({2})", "{0} carpetas, {1} archivos ({2})"],
    opening => ["A abrir {0}...", "Opening {0}...", "Ouverture de {0}...", "Abriendo {0}..."],
    err_root => ["Erro ao abrir a pasta raiz.", "Error opening the root folder.", "Erreur à l'ouverture du dossier racine.", "Error al abrir la carpeta raíz."],
    err_open_volume => ["Não foi possível abrir", "Could not open", "Impossible d'ouvrir", "No se pudo abrir"],
    err_list => ["Erro ao listar a pasta", "Error listing the folder", "Erreur lors du listage du dossier", "Error al listar la carpeta"],
    error => ["Erro", "Error", "Erreur", "Error"],
    file_title => ["Ficheiro", "File", "Fichier", "Archivo"],
    file_info => [
        "{0}\n\nTamanho: {1} ({2} bytes)\nModificado: {3}\nCriado: {4}\nPermissões: {5}{6}\n\nSelecione o ficheiro e use \"Copiar selecionados...\", ou monte o volume como unidade para o abrir diretamente.",
        "{0}\n\nSize: {1} ({2} bytes)\nModified: {3}\nCreated: {4}\nPermissions: {5}{6}\n\nSelect the file and use \"Copy selected...\", or mount the volume as a drive to open it directly.",
        "{0}\n\nTaille : {1} ({2} octets)\nModifié : {3}\nCréé : {4}\nPermissions : {5}{6}\n\nSélectionnez le fichier et utilisez « Copier la sélection... », ou montez le volume comme lecteur pour l'ouvrir directement.",
        "{0}\n\nTamaño: {1} ({2} bytes)\nModificado: {3}\nCreado: {4}\nPermisos: {5}{6}\n\nSeleccione el archivo y use \"Copiar seleccionados...\", o monte el volumen como unidad para abrirlo directamente."
    ],
    file_compressed_note => ["\nComprimido pelo macOS (será descomprimido na cópia)", "\nCompressed by macOS (will be decompressed when copied)", "\nCompressé par macOS (sera décompressé à la copie)", "\nComprimido por macOS (se descomprimirá al copiar)"],
    symlink_title => ["Link simbólico", "Symbolic link", "Lien symbolique", "Enlace simbólico"],

    // ---- copiar / verificar / montar
    copy_title => ["Copiar", "Copy", "Copier", "Copiar"],
    verify_title => ["Verificar", "Verify", "Vérifier", "Verificar"],
    mount_title => ["Montar", "Mount", "Monter", "Montar"],
    need_volume => ["Abra primeiro um volume: selecione-o no ecrã inicial e clique em \"Abrir no programa\".", "Open a volume first: select it on the start screen and click \"Open in the program\".", "Ouvrez d'abord un volume : sélectionnez-le sur l'écran d'accueil et cliquez sur « Ouvrir dans le programme ».", "Abra primero un volumen: selecciónelo en la pantalla inicial y pulse \"Abrir en el programa\"."],
    select_items => ["Selecione um ou mais itens na lista (Ctrl/Shift para vários).", "Select one or more items in the list (Ctrl/Shift for several).", "Sélectionnez un ou plusieurs éléments dans la liste (Ctrl/Maj pour plusieurs).", "Seleccione uno o más elementos de la lista (Ctrl/Mayús para varios)."],
    folder_dialog => ["Escolha a pasta de destino no Windows", "Choose the destination folder on Windows", "Choisissez le dossier de destination sous Windows", "Elija la carpeta de destino en Windows"],
    image_dialog => ["Abrir imagem de disco ou DMG", "Open disk image or DMG", "Ouvrir une image disque ou DMG", "Abrir imagen de disco o DMG"],
    image_filter => ["Imagens de disco(*.img;*.raw;*.dd;*.dmg;*.bin;*.iso;*.hfs;*.apfs)|Todos os ficheiros(*.*)", "Disk images(*.img;*.raw;*.dd;*.dmg;*.bin;*.iso;*.hfs;*.apfs)|All files(*.*)", "Images disque(*.img;*.raw;*.dd;*.dmg;*.bin;*.iso;*.hfs;*.apfs)|Tous les fichiers(*.*)", "Imágenes de disco(*.img;*.raw;*.dd;*.dmg;*.bin;*.iso;*.hfs;*.apfs)|Todos los archivos(*.*)"],
    copying => ["A copiar...", "Copying...", "Copie en cours...", "Copiando..."],
    verifying => ["A verificar a leitura...", "Verifying read...", "Vérification de la lecture...", "Verificando la lectura..."],
    cancelling => ["A cancelar...", "Cancelling...", "Annulation...", "Cancelando..."],
    progress => ["{0} ficheiros, {1}  —  {2}", "{0} files, {1}  —  {2}", "{0} fichiers, {1}  —  {2}", "{0} archivos, {1}  —  {2}"],
    failed => ["Falhou.", "Failed.", "Échec.", "Falló."],
    failure => ["Falha", "Failure", "Échec", "Fallo"],
    first_errors => ["\n\nPrimeiros erros:\n", "\n\nFirst errors:\n", "\n\nPremières erreurs :\n", "\n\nPrimeros errores:\n"],
    log_full => ["\nLog completo: {0}", "\nFull log: {0}", "\nJournal complet : {0}", "\nRegistro completo: {0}"],
    done => ["Concluído", "Done", "Terminé", "Completado"],
    done_errors => ["Concluído com erros", "Done with errors", "Terminé avec des erreurs", "Completado con errores"],
    summary_copy => ["Cópia", "Copy", "Copie", "Copia"],
    summary_verify => ["Verificação", "Verification", "Vérification", "Verificación"],
    summary => ["{0}{1}: {2} ficheiros ({3}) e {4} pastas em {5} s ({6}/s). {7} já existiam no destino, {8} links, {9} erros.", "{0}{1}: {2} files ({3}) and {4} folders in {5} s ({6}/s). {7} already existed at destination, {8} links, {9} errors.", "{0}{1} : {2} fichiers ({3}) et {4} dossiers en {5} s ({6}/s). {7} existaient déjà, {8} liens, {9} erreurs.", "{0}{1}: {2} archivos ({3}) y {4} carpetas en {5} s ({6}/s). {7} ya existían en el destino, {8} enlaces, {9} errores."],
    cancelled_prefix => ["Cancelado. ", "Cancelled. ", "Annulé. ", "Cancelado. "],
    dest_create_fail => ["não foi possível criar a pasta de destino: {0}", "could not create the destination folder: {0}", "impossible de créer le dossier de destination : {0}", "no se pudo crear la carpeta de destino: {0}"],
    internal_error => ["erro interno (metadados corrompidos?)", "internal error (corrupted metadata?)", "erreur interne (métadonnées corrompues ?)", "error interno (¿metadatos dañados?)"],
    exit_title => ["Sair", "Exit", "Quitter", "Salir"],
    exit_text => ["Há uma cópia em andamento. Deseja cancelar e sair?", "A copy is in progress. Cancel it and exit?", "Une copie est en cours. Annuler et quitter ?", "Hay una copia en curso. ¿Cancelar y salir?"],
    no_admin_title => ["Sem privilégios de Administrador", "No Administrator privileges", "Pas de privilèges Administrateur", "Sin privilegios de Administrador"],
    no_admin_text => ["O programa não está a correr como Administrador, por isso os discos físicos não podem ser lidos.\nImagens de disco e DMG continuam a funcionar.\n\nFeche e execute de novo aceitando o pedido de permissão para aceder aos discos.", "The program is not running as Administrator, so physical disks cannot be read.\nDisk images and DMG files still work.\n\nClose it and run it again accepting the permission request to access the disks.", "Le programme ne s'exécute pas en tant qu'Administrateur, les disques physiques ne peuvent donc pas être lus.\nLes images disque et DMG fonctionnent toujours.\n\nFermez-le et relancez-le en acceptant la demande de permission.", "El programa no se ejecuta como Administrador, por lo que los discos físicos no pueden leerse.\nLas imágenes de disco y DMG siguen funcionando.\n\nCiérrelo y ejecútelo de nuevo aceptando la solicitud de permiso."],
    dokan_needed_title => ["Driver Dokan necessário", "Dokan driver required", "Pilote Dokan requis", "Se necesita el controlador Dokan"],
    dokan_needed_text => ["Para montar o volume como uma unidade do Windows é preciso o driver Dokan 2 (gratuito, código aberto).\n\n{0}\n\nO instalador do SUDOMAKE Partition oferece a instalação do Dokan; também pode descarregá-lo em https://github.com/dokan-dev/dokany/releases (DokanSetup.exe). As outras funções (navegar e copiar) não precisam dele.", "Mounting the volume as a Windows drive requires the Dokan 2 driver (free, open source).\n\n{0}\n\nThe SUDOMAKE Partition installer offers to install Dokan; you can also download it from https://github.com/dokan-dev/dokany/releases (DokanSetup.exe). The other features (browse and copy) do not need it.", "Monter le volume comme lecteur Windows nécessite le pilote Dokan 2 (gratuit, open source).\n\n{0}\n\nL'installateur de SUDOMAKE Partition propose d'installer Dokan ; vous pouvez aussi le télécharger sur https://github.com/dokan-dev/dokany/releases (DokanSetup.exe). Les autres fonctions (parcourir et copier) n'en ont pas besoin.", "Montar el volumen como unidad de Windows requiere el controlador Dokan 2 (gratuito, código abierto).\n\n{0}\n\nEl instalador de SUDOMAKE Partition ofrece instalar Dokan; también puede descargarlo en https://github.com/dokan-dev/dokany/releases (DokanSetup.exe). Las demás funciones (navegar y copiar) no lo necesitan."],
    no_free_letter => ["Não há letra de unidade livre.", "No free drive letter.", "Aucune lettre de lecteur libre.", "No hay letra de unidad libre."],
    mounting => ["A montar como {0}:...", "Mounting as {0}:...", "Montage comme {0}:...", "Montando como {0}:..."],
    mounted_status => ["Unidade {0}: montada (somente leitura). Clique em \"Desmontar\" antes de remover o disco.", "Drive {0}: mounted (read-only). Click \"Unmount\" before removing the disk.", "Lecteur {0}: monté (lecture seule). Cliquez sur « Démonter » avant de retirer le disque.", "Unidad {0}: montada (solo lectura). Pulse \"Desmontar\" antes de retirar el disco."],
    unmounted_status => ["Unidade {0}: desmontada.", "Drive {0}: unmounted.", "Lecteur {0}: démonté.", "Unidad {0}: desmontada."],
    mount_failed => ["Falha ao montar.", "Mount failed.", "Échec du montage.", "Error al montar."],
    mount_failed_title => ["Não foi possível montar", "Could not mount", "Impossible de monter", "No se pudo montar"],
    select_to_mount => ["Selecione no ecrã inicial o volume que deseja montar como unidade.", "Select on the start screen the volume you want to mount as a drive.", "Sélectionnez sur l'écran d'accueil le volume à monter comme lecteur.", "Seleccione en la pantalla inicial el volumen que desea montar como unidad."],

    // ---- marketing / sobre / doação
    made_in_angola => ["Feito em Angola", "Made in Angola", "Fabriqué en Angola", "Hecho en Angola"],
    btn_whatsapp => ["WhatsApp +244 932 693 623", "WhatsApp +244 932 693 623", "WhatsApp +244 932 693 623", "WhatsApp +244 932 693 623"],
    btn_email => ["E-mail", "E-mail", "E-mail", "Correo"],
    btn_site => ["sudomakes.com", "sudomakes.com", "sudomakes.com", "sudomakes.com"],
    btn_youtube => ["YouTube", "YouTube", "YouTube", "YouTube"],
    btn_github => ["GitHub", "GitHub", "GitHub", "GitHub"],
    btn_paypal => ["Doar via PayPal", "Donate via PayPal", "Don via PayPal", "Donar por PayPal"],
    about_title => ["Sobre o SUDOMAKE Partition", "About SUDOMAKE Partition", "À propos de SUDOMAKE Partition", "Acerca de SUDOMAKE Partition"],
    about_text => [
        "SUDOMAKE Partition {0}\r\n\r\nO QUE FAZ\r\nLê e copia ficheiros de discos de Mac (APFS, HFS+/HFSX) e Linux (ext2/3/4, também dentro de LVM) diretamente no Windows, sem instalar drivers, incluindo imagens de disco (.img, .raw) e DMG. Mostra o sistema instalado em cada partição, copia pastas inteiras com registo de erros e retomada, e pode montar um volume como uma letra de unidade (somente leitura) através do driver Dokan 2. Tudo é somente leitura: nada é gravado no disco de origem.\r\n\r\nQUEM DESENVOLVEU\r\n{1}, em Angola.\r\nEmpresa: {2} · NIF {3}\r\nWhatsApp: {4} · E-mail: {5}\r\nSite: {6} · YouTube: {7} · GitHub: {8}\r\n\r\nCÓDIGO ABERTO\r\nLicença MIT. Código-fonte e versões: {9}\r\n\r\nDOAÇÃO (não obrigatória)\r\nPayPal: {5}\r\nTransferência Express (Angola): {10}\r\nDepois de doar, envie o comprovativo pelo WhatsApp ou e-mail. Obrigado!",
        "SUDOMAKE Partition {0}\r\n\r\nWHAT IT DOES\r\nReads and copies files from Mac (APFS, HFS+/HFSX) and Linux (ext2/3/4, also inside LVM) disks directly on Windows, without installing drivers, including disk images (.img, .raw) and DMG files. Shows the system installed on each partition, copies whole folders with error logging and resume, and can mount a volume as a read-only drive letter through the Dokan 2 driver. Everything is read-only: nothing is written to the source disk.\r\n\r\nWHO MADE IT\r\n{1}, in Angola.\r\nCompany: {2} · VAT {3}\r\nWhatsApp: {4} · E-mail: {5}\r\nWebsite: {6} · YouTube: {7} · GitHub: {8}\r\n\r\nOPEN SOURCE\r\nMIT license. Source code and releases: {9}\r\n\r\nDONATION (optional)\r\nPayPal: {5}\r\nExpress transfer (Angola): {10}\r\nAfter donating, send the receipt by WhatsApp or e-mail. Thank you!",
        "SUDOMAKE Partition {0}\r\n\r\nCE QU'IL FAIT\r\nLit et copie des fichiers depuis des disques Mac (APFS, HFS+/HFSX) et Linux (ext2/3/4, aussi dans LVM) directement sous Windows, sans installer de pilotes, y compris les images disque (.img, .raw) et DMG. Affiche le système installé sur chaque partition, copie des dossiers entiers avec journal des erreurs et reprise, et peut monter un volume comme lettre de lecteur (lecture seule) via le pilote Dokan 2. Tout est en lecture seule : rien n'est écrit sur le disque d'origine.\r\n\r\nAUTEUR\r\n{1}, en Angola.\r\nEntreprise : {2} · NIF {3}\r\nWhatsApp : {4} · E-mail : {5}\r\nSite : {6} · YouTube : {7} · GitHub : {8}\r\n\r\nOPEN SOURCE\r\nLicence MIT. Code source et versions : {9}\r\n\r\nDON (facultatif)\r\nPayPal : {5}\r\nTransfert Express (Angola) : {10}\r\nAprès un don, envoyez le reçu par WhatsApp ou e-mail. Merci !",
        "SUDOMAKE Partition {0}\r\n\r\nQUÉ HACE\r\nLee y copia archivos de discos Mac (APFS, HFS+/HFSX) y Linux (ext2/3/4, también dentro de LVM) directamente en Windows, sin instalar controladores, incluidas imágenes de disco (.img, .raw) y DMG. Muestra el sistema instalado en cada partición, copia carpetas completas con registro de errores y reanudación, y puede montar un volumen como letra de unidad (solo lectura) mediante el controlador Dokan 2. Todo es solo lectura: no se escribe nada en el disco de origen.\r\n\r\nQUIÉN LO DESARROLLÓ\r\n{1}, en Angola.\r\nEmpresa: {2} · NIF {3}\r\nWhatsApp: {4} · Correo: {5}\r\nSitio: {6} · YouTube: {7} · GitHub: {8}\r\n\r\nCÓDIGO ABIERTO\r\nLicencia MIT. Código fuente y versiones: {9}\r\n\r\nDONACIÓN (opcional)\r\nPayPal: {5}\r\nTransferencia Express (Angola): {10}\r\nTras donar, envíe el comprobante por WhatsApp o correo. ¡Gracias!"
    ],
    donate_title => ["Apoie o desenvolvedor ❤", "Support the developer ❤", "Soutenez le développeur ❤", "Apoye al desarrollador ❤"],
    donate_text => [
        "O SUDOMAKE Partition é gratuito, de código aberto e feito em Angola. Se ele o ajudou a recuperar os seus ficheiros, considere fazer uma doação de qualquer valor (não é obrigatória):\r\n\r\n•  PayPal: {0}\r\n•  Transferência Express (Angola): {1}\r\n\r\nDepois de doar, envie o comprovativo pelo WhatsApp ({2}) ou por e-mail ({0}) para podermos agradecer.\r\n\r\nSiga também o canal no YouTube ({3}) e o GitHub ({4}) para acompanhar novas versões e outros projetos.\r\n\r\nUse os botões abaixo para abrir o WhatsApp, o e-mail, o site, o YouTube, o GitHub ou o PayPal.",
        "SUDOMAKE Partition is free, open source and made in Angola. If it helped you recover your files, please consider a donation of any amount (optional):\r\n\r\n•  PayPal: {0}\r\n•  Express transfer (Angola): {1}\r\n\r\nAfter donating, send the receipt by WhatsApp ({2}) or e-mail ({0}) so we can thank you.\r\n\r\nAlso follow the YouTube channel ({3}) and GitHub ({4}) for new versions and other projects.\r\n\r\nUse the buttons below to open WhatsApp, e-mail, the website, YouTube, GitHub or PayPal.",
        "SUDOMAKE Partition est gratuit, open source et fabriqué en Angola. S'il vous a aidé à récupérer vos fichiers, pensez à faire un don du montant de votre choix (facultatif) :\r\n\r\n•  PayPal : {0}\r\n•  Transfert Express (Angola) : {1}\r\n\r\nAprès un don, envoyez le reçu par WhatsApp ({2}) ou par e-mail ({0}) afin que nous puissions vous remercier.\r\n\r\nSuivez aussi la chaîne YouTube ({3}) et le GitHub ({4}) pour les nouvelles versions et d'autres projets.\r\n\r\nUtilisez les boutons ci-dessous pour ouvrir WhatsApp, l'e-mail, le site, YouTube, GitHub ou PayPal.",
        "SUDOMAKE Partition es gratuito, de código abierto y hecho en Angola. Si le ayudó a recuperar sus archivos, considere hacer una donación de cualquier importe (opcional):\r\n\r\n•  PayPal: {0}\r\n•  Transferencia Express (Angola): {1}\r\n\r\nTras donar, envíe el comprobante por WhatsApp ({2}) o por correo ({0}) para poder agradecerle.\r\n\r\nSiga también el canal de YouTube ({3}) y el GitHub ({4}) para conocer nuevas versiones y otros proyectos.\r\n\r\nUse los botones de abajo para abrir WhatsApp, el correo, el sitio, YouTube, GitHub o PayPal."
    ],
}

/// Substitui {0}, {1}... pelos argumentos.
pub fn fmt(template: &str, args: &[&str]) -> String {
    let mut s = template.to_string();
    for (i, a) in args.iter().enumerate() {
        s = s.replace(&format!("{{{}}}", i), a);
    }
    s
}
