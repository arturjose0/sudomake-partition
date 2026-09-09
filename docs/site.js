/* SUDOMAKE Partition — site: idiomas, release e contribuidores em tempo real (API do GitHub),
   apoiantes (donors.json), galeria. Sem dependências. */
(function () {
  var REPO = 'arturjose0/sudomake-partition';
  var API = 'https://api.github.com/repos/' + REPO;

  var T = {
    pt: {
      nav_features: 'Funcionalidades', nav_shots: 'Capturas', nav_howto: 'Como usar', nav_download: 'Download', nav_contrib: 'Contribuidores', nav_donate: 'Doar ❤',
      made_in: 'Feito em Angola',
      hero_sub: 'Ler, copiar e montar discos de Mac e Linux no Windows',
      hero_lead: 'O Windows não abre discos de Mac (APFS, HFS+) nem de Linux (ext4, LVM). O SUDOMAKE Partition abre: mostra as partições em azulejos, diz que sistema está em cada uma, e deixa-o copiar os ficheiros ou montar o disco como uma letra de unidade. Grátis, de código aberto e somente leitura: nada é alterado no disco original.',
      btn_download: 'Descarregar o instalador', btn_zip: 'Versão portátil (zip)',
      dl_meta: 'Windows 10/11 de 64 bits · sem requisitos prévios · licença MIT',
      f_title: 'O que ele faz',
      f_sub: 'Serve para o disco interno retirado de um MacBook, iMac ou PC com Ubuntu, Debian, Kali ou Fedora, para discos externos e pen drives formatados no Mac ou no Linux, e para imagens de disco.',
      f1t: 'Discos de Mac', f1: 'APFS (macOS 10.13 até ao mais recente, todos os volumes, ficheiros comprimidos) e HFS+/HFSX (Mac OS Extended, Time Machine).',
      f2t: 'Discos de Linux', f2: 'ext4, ext3 e ext2, também dentro de volumes LVM. Ubuntu, Debian, Kali, Mint, Fedora, RHEL…',
      f3t: 'Imagens de disco', f3: 'DMG (zlib, bzip2, ADC, LZFSE, LZMA), .img, .raw e .dd, com tabelas GPT, MBR e Apple Partition Map.',
      f4t: 'Detecta o sistema', f4: 'Cada partição mostra o que tem: "Ubuntu 22.04", "Kali Linux", "macOS 12.5", "Windows", "EFI", "swap", "Time Machine"…',
      f5t: 'Montar como unidade', f5: 'O volume vira uma letra (Z:), somente leitura, visível no Explorador e em qualquer programa. Vários discos ao mesmo tempo.',
      f6t: 'Copiar tudo com segurança', f6: 'Copie pastas ou o disco completo com progresso, retomada e registo de erros. Antes de copiar um disco inteiro, verifica se há espaço no destino.',
      f7t: 'Somente leitura', f7: 'Nunca escreve, repara nem altera o disco de origem. Não precisa de senha para discos comuns (só os cifrados ficam de fora).',
      f8t: '4 idiomas e temas', f8: 'Português (Angola), English, Français, Español. Tema claro, escuro ou do sistema. Os discos ligados ou removidos aparecem sozinhos.',
      s_title: 'Capturas de ecrã', s_sub: 'Imagens da versão actual. Clique para ampliar.',
      s1: 'Ecrã inicial: os discos e as partições em azulejos, com o sistema encontrado em cada uma e o espaço usado.',
      s2: 'Ao clicar numa partição, a barra mostra só os botões que fazem sentido: abrir, montar, copiar o disco completo, verificar.',
      s3: 'Dois discos montados ao mesmo tempo como Z: e Y:, cada um com a sua etiqueta.',
      s4: 'Navegação dentro de uma partição Linux: pastas, ficheiros, links e datas, com cópia dos itens seleccionados.',
      s5: 'Tema escuro (o padrão segue o Windows).', s6: 'Interface em inglês; também em francês e espanhol.',
      s7: 'Página "Sobre": o que faz, quem desenvolveu e como apoiar.', s8: 'Instalador em cinco idiomas, que descarrega e instala o driver Dokan sozinho se faltar.',
      h_title: 'Como usar',
      h1t: 'Instale', h1: 'Descarregue o instalador e siga o assistente. Se o driver Dokan (só para montar unidades) faltar, o instalador trata disso.',
      h2t: 'Ligue o disco e abra', h2: 'Ligue o disco por USB ou adaptador e abra o programa. Aceite o pedido de Administrador: o Windows exige-o para ler discos físicos.',
      h3t: 'Clique na partição', h3: 'Os ficheiros pessoais estão em /Users/nome (Mac, volume "Macintosh HD - Data") ou /home/nome (Linux).',
      h4t: 'Abra, copie ou monte', h4: 'Abrir no programa para navegar e copiar; Montar como unidade para usar o disco em qualquer programa; Copiar disco completo para levar tudo para outro disco.',
      d_title: 'Download', d_latest: 'Última versão', d_loading: 'A obter informação da release no GitHub…', d_all: 'Todas as versões',
      d_published: 'Publicada em', d_downloads: 'transferências', d_fail: 'Não foi possível contactar o GitHub agora. Use o botão acima: leva sempre à última versão.',
      d_req_t: 'Requisitos e notas',
      d_req1: 'Windows 10 ou 11 de 64 bits. Nada mais precisa de estar instalado.',
      d_req2: 'O instalador (recomendado) instala para o seu utilizador ou para todos, cria atalhos e desinstalador, e descarrega o driver Dokan 2 se faltar.',
      d_req3: 'A versão portátil (zip) não precisa de instalação: descompacte e execute SudomakePartition.exe.',
      d_req4: 'Ler um disco físico exige permissão de Administrador; o programa pede-a ao abrir.',
      d_req5: 'Não abre discos cifrados (FileVault, LUKS, eCryptfs) nem Macs com chip T2 ou Apple Silicon.',
      c_title: 'Contribuidores', c_sub: 'Lista obtida em tempo real do GitHub. Toda a ajuda é bem-vinda: relatos de teste com discos reais, traduções, documentação e código.',
      c_loading: 'A carregar…', c_fail: 'Não foi possível carregar a lista agora. Veja no GitHub.', c_commits: 'contribuições', c_how: 'Como contribuir', c_issue: 'Reportar um problema',
      g_title: 'Apoie o desenvolvedor',
      g_sub: 'O SUDOMAKE Partition é gratuito e vai continuar a ser. Se ele o ajudou a recuperar os seus ficheiros, uma doação de qualquer valor ajuda a manter e melhorar o projecto. Não é obrigatória.',
      g_how: 'Como doar', g_paypal: 'Doar via PayPal', g_express: 'Transferência Express (Angola):',
      g_after: 'Depois de doar, envie o comprovativo pelo WhatsApp ou e-mail para podermos agradecer e, se quiser, incluir o seu nome na lista de apoiantes.',
      g_yt: 'Seguir no YouTube', g_list: 'Apoiantes', g_empty: 'Ainda ninguém. Seja o primeiro apoiante!', g_thanks: 'Obrigado a todos!',
      a_title: 'Quem desenvolveu',
      a_p1: 'O SUDOMAKE Partition foi criado em Angola por <b>José Artur Kassala</b>, através da sua empresa <b>SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA</b>. Nasceu de uma necessidade real: recuperar ficheiros de discos de Mac e de Linux usando apenas um computador com Windows, sem pagar por programas fechados.',
      a_p2: 'É escrito em Rust, lê os sistemas de ficheiros directamente (sem drivers), e foi validado byte a byte contra o 7-Zip e contra discos reais. O código é aberto sob licença MIT: qualquer pessoa pode usar, estudar, melhorar e distribuir.',
      a_company: 'Empresa', a_country: 'País', f_license: 'Licença MIT', f_manual: 'Manual'
    },
    en: {
      nav_features: 'Features', nav_shots: 'Screenshots', nav_howto: 'How to use', nav_download: 'Download', nav_contrib: 'Contributors', nav_donate: 'Donate ❤',
      made_in: 'Made in Angola',
      hero_sub: 'Read, copy and mount Mac and Linux disks on Windows',
      hero_lead: 'Windows cannot open Mac (APFS, HFS+) or Linux (ext4, LVM) disks. SUDOMAKE Partition can: it shows the partitions as tiles, tells you which system is on each one, and lets you copy the files or mount the disk as a drive letter. Free, open source and strictly read-only: nothing on the original disk is changed.',
      btn_download: 'Download the installer', btn_zip: 'Portable version (zip)',
      dl_meta: '64-bit Windows 10/11 · no prerequisites · MIT license',
      f_title: 'What it does',
      f_sub: 'For the internal disk taken out of a MacBook, iMac or a PC running Ubuntu, Debian, Kali or Fedora, for external disks and USB sticks formatted on Mac or Linux, and for disk images.',
      f1t: 'Mac disks', f1: 'APFS (macOS 10.13 to the latest, all volumes, compressed files) and HFS+/HFSX (Mac OS Extended, Time Machine).',
      f2t: 'Linux disks', f2: 'ext4, ext3 and ext2, also inside LVM volumes. Ubuntu, Debian, Kali, Mint, Fedora, RHEL…',
      f3t: 'Disk images', f3: 'DMG (zlib, bzip2, ADC, LZFSE, LZMA), .img, .raw and .dd, with GPT, MBR and Apple Partition Map tables.',
      f4t: 'Detects the system', f4: 'Every partition shows what it holds: "Ubuntu 22.04", "Kali Linux", "macOS 12.5", "Windows", "EFI", "swap", "Time Machine"…',
      f5t: 'Mount as a drive', f5: 'The volume becomes a drive letter (Z:), read-only, visible in Explorer and in any program. Several disks at the same time.',
      f6t: 'Copy everything safely', f6: 'Copy folders or the entire disk with progress, resume and an error log. Before copying a whole disk it checks the free space at the destination.',
      f7t: 'Read-only', f7: 'It never writes to, repairs or changes the source disk. No password needed for ordinary disks (only encrypted ones are excluded).',
      f8t: '4 languages and themes', f8: 'Portuguese (Angola), English, French, Spanish. Light, dark or system theme. Disks plugged in or removed appear on their own.',
      s_title: 'Screenshots', s_sub: 'Images from the current version. Click to enlarge.',
      s1: 'Start screen: disks and partitions as tiles, with the system found on each one and the used space.',
      s2: 'Clicking a partition shows only the buttons that apply: open, mount, copy the entire disk, verify.',
      s3: 'Two disks mounted at the same time as Z: and Y:, each with its badge.',
      s4: 'Browsing inside a Linux partition: folders, files, links and dates, copying the selected items.',
      s5: 'Dark theme (the default follows Windows).', s6: 'Interface in English; also in French and Spanish.',
      s7: '"About" page: what it does, who made it and how to support it.', s8: 'Installer in five languages; downloads and installs the Dokan driver by itself if missing.',
      h_title: 'How to use',
      h1t: 'Install', h1: 'Download the installer and follow the wizard. If the Dokan driver (only needed to mount drives) is missing, the installer takes care of it.',
      h2t: 'Plug in the disk and open', h2: 'Connect the disk via USB or an adapter and open the program. Accept the Administrator prompt: Windows requires it to read physical disks.',
      h3t: 'Click the partition', h3: 'Personal files live in /Users/name (Mac, "Macintosh HD - Data" volume) or /home/name (Linux).',
      h4t: 'Open, copy or mount', h4: 'Open in the program to browse and copy; Mount as drive to use the disk in any program; Copy entire disk to move everything to another disk.',
      d_title: 'Download', d_latest: 'Latest version', d_loading: 'Fetching release information from GitHub…', d_all: 'All versions',
      d_published: 'Published on', d_downloads: 'downloads', d_fail: 'GitHub could not be reached right now. Use the button above: it always leads to the latest version.',
      d_req_t: 'Requirements and notes',
      d_req1: '64-bit Windows 10 or 11. Nothing else needs to be installed.',
      d_req2: 'The installer (recommended) installs for your user or for everyone, creates shortcuts and an uninstaller, and downloads the Dokan 2 driver if missing.',
      d_req3: 'The portable version (zip) needs no installation: unzip and run SudomakePartition.exe.',
      d_req4: 'Reading a physical disk requires Administrator rights; the program asks for them when it opens.',
      d_req5: 'It does not open encrypted disks (FileVault, LUKS, eCryptfs) or T2 / Apple Silicon Macs.',
      c_title: 'Contributors', c_sub: 'List fetched live from GitHub. All help is welcome: test reports with real disks, translations, documentation and code.',
      c_loading: 'Loading…', c_fail: 'The list could not be loaded right now. See it on GitHub.', c_commits: 'contributions', c_how: 'How to contribute', c_issue: 'Report a problem',
      g_title: 'Support the developer',
      g_sub: 'SUDOMAKE Partition is free and will stay free. If it helped you recover your files, a donation of any amount helps to maintain and improve the project. It is not required.',
      g_how: 'How to donate', g_paypal: 'Donate via PayPal', g_express: 'Express transfer (Angola):',
      g_after: 'After donating, send the receipt via WhatsApp or e-mail so we can thank you and, if you wish, add your name to the supporters list.',
      g_yt: 'Follow on YouTube', g_list: 'Supporters', g_empty: 'Nobody yet. Be the first supporter!', g_thanks: 'Thank you all!',
      a_title: 'Who made it',
      a_p1: 'SUDOMAKE Partition was created in Angola by <b>José Artur Kassala</b> through his company <b>SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA</b>. It was born from a real need: recovering files from Mac and Linux disks using only a Windows computer, without paying for closed software.',
      a_p2: 'It is written in Rust, reads the file systems directly (no drivers), and was validated byte by byte against 7-Zip and against real disks. The code is open under the MIT license: anyone can use, study, improve and distribute it.',
      a_company: 'Company', a_country: 'Country', f_license: 'MIT License', f_manual: 'Manual'
    },
    fr: {
      nav_features: 'Fonctionnalités', nav_shots: 'Captures', nav_howto: 'Utilisation', nav_download: 'Télécharger', nav_contrib: 'Contributeurs', nav_donate: 'Faire un don ❤',
      made_in: 'Fabriqué en Angola',
      hero_sub: 'Lire, copier et monter des disques Mac et Linux sous Windows',
      hero_lead: 'Windows n\'ouvre pas les disques Mac (APFS, HFS+) ni Linux (ext4, LVM). SUDOMAKE Partition le fait : il affiche les partitions sous forme de tuiles, indique le système présent sur chacune, et vous laisse copier les fichiers ou monter le disque comme une lettre de lecteur. Gratuit, open source et en lecture seule : rien n\'est modifié sur le disque d\'origine.',
      btn_download: 'Télécharger l\'installateur', btn_zip: 'Version portable (zip)',
      dl_meta: 'Windows 10/11 64 bits · aucun prérequis · licence MIT',
      f_title: 'Ce qu\'il fait',
      f_sub: 'Pour le disque interne retiré d\'un MacBook, d\'un iMac ou d\'un PC sous Ubuntu, Debian, Kali ou Fedora, pour les disques externes et clés USB formatés sur Mac ou Linux, et pour les images de disque.',
      f1t: 'Disques Mac', f1: 'APFS (macOS 10.13 jusqu\'au plus récent, tous les volumes, fichiers compressés) et HFS+/HFSX (Mac OS Étendu, Time Machine).',
      f2t: 'Disques Linux', f2: 'ext4, ext3 et ext2, y compris dans des volumes LVM. Ubuntu, Debian, Kali, Mint, Fedora, RHEL…',
      f3t: 'Images de disque', f3: 'DMG (zlib, bzip2, ADC, LZFSE, LZMA), .img, .raw et .dd, avec tables GPT, MBR et Apple Partition Map.',
      f4t: 'Détecte le système', f4: 'Chaque partition indique ce qu\'elle contient : « Ubuntu 22.04 », « Kali Linux », « macOS 12.5 », « Windows », « EFI », « swap », « Time Machine »…',
      f5t: 'Monter comme lecteur', f5: 'Le volume devient une lettre (Z:), en lecture seule, visible dans l\'Explorateur et dans n\'importe quel programme. Plusieurs disques à la fois.',
      f6t: 'Tout copier en sécurité', f6: 'Copiez des dossiers ou le disque entier avec progression, reprise et journal des erreurs. Avant de copier un disque entier, l\'espace libre à destination est vérifié.',
      f7t: 'Lecture seule', f7: 'Il n\'écrit, ne répare ni ne modifie jamais le disque source. Aucun mot de passe nécessaire pour les disques ordinaires (seuls les disques chiffrés sont exclus).',
      f8t: '4 langues et thèmes', f8: 'Portugais (Angola), anglais, français, espagnol. Thème clair, sombre ou système. Les disques branchés ou retirés apparaissent automatiquement.',
      s_title: 'Captures d\'écran', s_sub: 'Images de la version actuelle. Cliquez pour agrandir.',
      s1: 'Écran d\'accueil : disques et partitions en tuiles, avec le système trouvé sur chacune et l\'espace utilisé.',
      s2: 'En cliquant sur une partition, la barre n\'affiche que les boutons utiles : ouvrir, monter, copier le disque entier, vérifier.',
      s3: 'Deux disques montés en même temps comme Z: et Y:, chacun avec son étiquette.',
      s4: 'Navigation dans une partition Linux : dossiers, fichiers, liens et dates, avec copie des éléments sélectionnés.',
      s5: 'Thème sombre (par défaut, celui de Windows).', s6: 'Interface en anglais ; aussi en français et en espagnol.',
      s7: 'Page « À propos » : ce qu\'il fait, qui l\'a créé et comment le soutenir.', s8: 'Installateur en cinq langues ; télécharge et installe le pilote Dokan tout seul s\'il manque.',
      h_title: 'Utilisation',
      h1t: 'Installez', h1: 'Téléchargez l\'installateur et suivez l\'assistant. Si le pilote Dokan (utile seulement pour monter des lecteurs) manque, l\'installateur s\'en occupe.',
      h2t: 'Branchez le disque et ouvrez', h2: 'Connectez le disque par USB ou adaptateur et ouvrez le programme. Acceptez la demande d\'Administrateur : Windows l\'exige pour lire les disques physiques.',
      h3t: 'Cliquez sur la partition', h3: 'Les fichiers personnels sont dans /Users/nom (Mac, volume « Macintosh HD - Data ») ou /home/nom (Linux).',
      h4t: 'Ouvrez, copiez ou montez', h4: 'Ouvrir dans le programme pour parcourir et copier ; Monter comme lecteur pour utiliser le disque partout ; Copier le disque entier pour tout transférer sur un autre disque.',
      d_title: 'Télécharger', d_latest: 'Dernière version', d_loading: 'Récupération des informations de la release sur GitHub…', d_all: 'Toutes les versions',
      d_published: 'Publiée le', d_downloads: 'téléchargements', d_fail: 'GitHub est injoignable pour le moment. Utilisez le bouton ci-dessus : il mène toujours à la dernière version.',
      d_req_t: 'Prérequis et remarques',
      d_req1: 'Windows 10 ou 11 64 bits. Rien d\'autre à installer.',
      d_req2: 'L\'installateur (recommandé) installe pour votre utilisateur ou pour tous, crée des raccourcis et un désinstallateur, et télécharge le pilote Dokan 2 s\'il manque.',
      d_req3: 'La version portable (zip) ne nécessite aucune installation : décompressez et lancez SudomakePartition.exe.',
      d_req4: 'Lire un disque physique exige les droits d\'Administrateur ; le programme les demande à l\'ouverture.',
      d_req5: 'Il n\'ouvre pas les disques chiffrés (FileVault, LUKS, eCryptfs) ni les Mac à puce T2 / Apple Silicon.',
      c_title: 'Contributeurs', c_sub: 'Liste obtenue en temps réel depuis GitHub. Toute aide est bienvenue : tests avec des disques réels, traductions, documentation et code.',
      c_loading: 'Chargement…', c_fail: 'La liste n\'a pas pu être chargée. Voir sur GitHub.', c_commits: 'contributions', c_how: 'Comment contribuer', c_issue: 'Signaler un problème',
      g_title: 'Soutenez le développeur',
      g_sub: 'SUDOMAKE Partition est gratuit et le restera. S\'il vous a aidé à récupérer vos fichiers, un don du montant de votre choix aide à maintenir et améliorer le projet. Ce n\'est pas obligatoire.',
      g_how: 'Comment faire un don', g_paypal: 'Don via PayPal', g_express: 'Transfert Express (Angola) :',
      g_after: 'Après votre don, envoyez le reçu par WhatsApp ou e-mail pour que nous puissions vous remercier et, si vous le souhaitez, ajouter votre nom à la liste des soutiens.',
      g_yt: 'Suivre sur YouTube', g_list: 'Soutiens', g_empty: 'Personne pour l\'instant. Soyez le premier !', g_thanks: 'Merci à tous !',
      a_title: 'Qui l\'a créé',
      a_p1: 'SUDOMAKE Partition a été créé en Angola par <b>José Artur Kassala</b>, via son entreprise <b>SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA</b>. Il est né d\'un besoin réel : récupérer des fichiers de disques Mac et Linux avec un simple ordinateur Windows, sans payer de logiciels fermés.',
      a_p2: 'Il est écrit en Rust, lit les systèmes de fichiers directement (sans pilote) et a été validé octet par octet face à 7-Zip et à des disques réels. Le code est ouvert sous licence MIT : chacun peut l\'utiliser, l\'étudier, l\'améliorer et le distribuer.',
      a_company: 'Entreprise', a_country: 'Pays', f_license: 'Licence MIT', f_manual: 'Manuel'
    },
    es: {
      nav_features: 'Funciones', nav_shots: 'Capturas', nav_howto: 'Cómo usar', nav_download: 'Descargar', nav_contrib: 'Colaboradores', nav_donate: 'Donar ❤',
      made_in: 'Hecho en Angola',
      hero_sub: 'Leer, copiar y montar discos Mac y Linux en Windows',
      hero_lead: 'Windows no abre discos de Mac (APFS, HFS+) ni de Linux (ext4, LVM). SUDOMAKE Partition sí: muestra las particiones en mosaicos, indica qué sistema hay en cada una y le permite copiar los archivos o montar el disco como una letra de unidad. Gratuito, de código abierto y de solo lectura: no se modifica nada en el disco original.',
      btn_download: 'Descargar el instalador', btn_zip: 'Versión portátil (zip)',
      dl_meta: 'Windows 10/11 de 64 bits · sin requisitos previos · licencia MIT',
      f_title: 'Qué hace',
      f_sub: 'Para el disco interno sacado de un MacBook, iMac o PC con Ubuntu, Debian, Kali o Fedora, para discos externos y memorias USB formateados en Mac o Linux, y para imágenes de disco.',
      f1t: 'Discos Mac', f1: 'APFS (macOS 10.13 hasta el más reciente, todos los volúmenes, archivos comprimidos) y HFS+/HFSX (Mac OS Plus, Time Machine).',
      f2t: 'Discos Linux', f2: 'ext4, ext3 y ext2, también dentro de volúmenes LVM. Ubuntu, Debian, Kali, Mint, Fedora, RHEL…',
      f3t: 'Imágenes de disco', f3: 'DMG (zlib, bzip2, ADC, LZFSE, LZMA), .img, .raw y .dd, con tablas GPT, MBR y Apple Partition Map.',
      f4t: 'Detecta el sistema', f4: 'Cada partición muestra lo que contiene: "Ubuntu 22.04", "Kali Linux", "macOS 12.5", "Windows", "EFI", "swap", "Time Machine"…',
      f5t: 'Montar como unidad', f5: 'El volumen se convierte en una letra (Z:), de solo lectura, visible en el Explorador y en cualquier programa. Varios discos a la vez.',
      f6t: 'Copiar todo con seguridad', f6: 'Copie carpetas o el disco completo con progreso, reanudación y registro de errores. Antes de copiar un disco entero comprueba el espacio libre en el destino.',
      f7t: 'Solo lectura', f7: 'Nunca escribe, repara ni altera el disco de origen. No necesita contraseña para discos normales (solo quedan fuera los cifrados).',
      f8t: '4 idiomas y temas', f8: 'Portugués (Angola), inglés, francés, español. Tema claro, oscuro o del sistema. Los discos conectados o retirados aparecen solos.',
      s_title: 'Capturas de pantalla', s_sub: 'Imágenes de la versión actual. Haga clic para ampliar.',
      s1: 'Pantalla inicial: discos y particiones en mosaicos, con el sistema encontrado en cada una y el espacio usado.',
      s2: 'Al hacer clic en una partición, la barra muestra solo los botones útiles: abrir, montar, copiar el disco completo, verificar.',
      s3: 'Dos discos montados a la vez como Z: e Y:, cada uno con su etiqueta.',
      s4: 'Navegación dentro de una partición Linux: carpetas, archivos, enlaces y fechas, con copia de los elementos seleccionados.',
      s5: 'Tema oscuro (el predeterminado sigue a Windows).', s6: 'Interfaz en inglés; también en francés y español.',
      s7: 'Página "Acerca de": qué hace, quién lo desarrolló y cómo apoyarlo.', s8: 'Instalador en cinco idiomas; descarga e instala el controlador Dokan por sí solo si falta.',
      h_title: 'Cómo usar',
      h1t: 'Instale', h1: 'Descargue el instalador y siga el asistente. Si falta el controlador Dokan (solo para montar unidades), el instalador se encarga.',
      h2t: 'Conecte el disco y abra', h2: 'Conecte el disco por USB o adaptador y abra el programa. Acepte la solicitud de Administrador: Windows lo exige para leer discos físicos.',
      h3t: 'Haga clic en la partición', h3: 'Los archivos personales están en /Users/nombre (Mac, volumen "Macintosh HD - Data") o /home/nombre (Linux).',
      h4t: 'Abra, copie o monte', h4: 'Abrir en el programa para navegar y copiar; Montar como unidad para usar el disco en cualquier programa; Copiar disco completo para llevarlo todo a otro disco.',
      d_title: 'Descargar', d_latest: 'Última versión', d_loading: 'Obteniendo información de la release en GitHub…', d_all: 'Todas las versiones',
      d_published: 'Publicada el', d_downloads: 'descargas', d_fail: 'No se pudo contactar con GitHub ahora. Use el botón de arriba: siempre lleva a la última versión.',
      d_req_t: 'Requisitos y notas',
      d_req1: 'Windows 10 u 11 de 64 bits. No hace falta instalar nada más.',
      d_req2: 'El instalador (recomendado) instala para su usuario o para todos, crea accesos directos y desinstalador, y descarga el controlador Dokan 2 si falta.',
      d_req3: 'La versión portátil (zip) no necesita instalación: descomprima y ejecute SudomakePartition.exe.',
      d_req4: 'Leer un disco físico exige permisos de Administrador; el programa los pide al abrir.',
      d_req5: 'No abre discos cifrados (FileVault, LUKS, eCryptfs) ni Macs con chip T2 o Apple Silicon.',
      c_title: 'Colaboradores', c_sub: 'Lista obtenida en tiempo real de GitHub. Toda ayuda es bienvenida: pruebas con discos reales, traducciones, documentación y código.',
      c_loading: 'Cargando…', c_fail: 'No se pudo cargar la lista ahora. Véala en GitHub.', c_commits: 'contribuciones', c_how: 'Cómo contribuir', c_issue: 'Informar de un problema',
      g_title: 'Apoye al desarrollador',
      g_sub: 'SUDOMAKE Partition es gratuito y lo seguirá siendo. Si le ayudó a recuperar sus archivos, una donación de cualquier importe ayuda a mantener y mejorar el proyecto. No es obligatoria.',
      g_how: 'Cómo donar', g_paypal: 'Donar por PayPal', g_express: 'Transferencia Express (Angola):',
      g_after: 'Después de donar, envíe el comprobante por WhatsApp o correo para poder agradecerle y, si lo desea, incluir su nombre en la lista de apoyos.',
      g_yt: 'Seguir en YouTube', g_list: 'Apoyos', g_empty: 'Todavía nadie. ¡Sea el primero!', g_thanks: '¡Gracias a todos!',
      a_title: 'Quién lo desarrolló',
      a_p1: 'SUDOMAKE Partition fue creado en Angola por <b>José Artur Kassala</b> a través de su empresa <b>SUDOMAKE - PRESTAÇÃO DE SERVIÇOS, (SU), LDA</b>. Nació de una necesidad real: recuperar archivos de discos Mac y Linux usando solo un ordenador con Windows, sin pagar por programas cerrados.',
      a_p2: 'Está escrito en Rust, lee los sistemas de archivos directamente (sin controladores) y se validó byte a byte contra 7-Zip y contra discos reales. El código es abierto bajo licencia MIT: cualquiera puede usarlo, estudiarlo, mejorarlo y distribuirlo.',
      a_company: 'Empresa', a_country: 'País', f_license: 'Licencia MIT', f_manual: 'Manual'
    }
  };

  var lang = 'pt';
  function t(k) { return (T[lang] && T[lang][k]) || T.pt[k] || k; }

  function applyLang(l) {
    lang = T[l] ? l : 'pt';
    try { localStorage.setItem('sp-lang', lang); } catch (e) {}
    document.documentElement.lang = lang;
    var nodes = document.querySelectorAll('[data-i18n]');
    for (var i = 0; i < nodes.length; i++) {
      var k = nodes[i].getAttribute('data-i18n');
      var v = t(k);
      if (k === 'a_p1' || k === 'a_p2') nodes[i].innerHTML = v; else nodes[i].textContent = v;
    }
    var btns = document.querySelectorAll('#lang button');
    for (var j = 0; j < btns.length; j++) btns[j].classList.toggle('on', btns[j].getAttribute('data-lang') === lang);
    renderRelease();
    renderContributors();
    renderDonors();
  }

  // ---- release mais recente ----
  var release = null, releaseFailed = false;
  function fmtSize(b) { if (b > 1e6) return (b / 1e6).toFixed(1) + ' MB'; if (b > 1e3) return (b / 1e3).toFixed(0) + ' KB'; return b + ' B'; }
  function renderRelease() {
    var name = document.getElementById('relName'), meta = document.getElementById('relMeta'), list = document.getElementById('assets');
    if (!release) { if (releaseFailed) { meta.textContent = t('d_fail'); } return; }
    var ver = release.tag_name.replace(/^v/, '');
    name.textContent = t('d_latest') + ': ' + ver;
    var date = new Date(release.published_at);
    var total = 0;
    (release.assets || []).forEach(function (a) { total += a.download_count || 0; });
    meta.textContent = t('d_published') + ' ' + date.toLocaleDateString(lang === 'en' ? 'en-US' : lang === 'fr' ? 'fr-FR' : lang === 'es' ? 'es-ES' : 'pt-PT') + ' · ' + total + ' ' + t('d_downloads');
    list.innerHTML = '';
    (release.assets || []).forEach(function (a) {
      var li = document.createElement('li');
      var link = document.createElement('a'); link.href = a.browser_download_url; link.textContent = a.name;
      var n = document.createElement('span'); n.className = 'n'; n.textContent = fmtSize(a.size) + ' · ' + (a.download_count || 0) + ' ' + t('d_downloads');
      li.appendChild(link); li.appendChild(n); list.appendChild(li);
      if (/setup.*\.exe$/i.test(a.name)) { document.getElementById('btnDownload').href = a.browser_download_url; document.getElementById('dlVersion').textContent = ver; }
      if (/\.zip$/i.test(a.name)) { document.getElementById('btnZip').href = a.browser_download_url; }
    });
  }
  fetch(API + '/releases/latest', { headers: { Accept: 'application/vnd.github+json' } })
    .then(function (r) { if (!r.ok) throw new Error(r.status); return r.json(); })
    .then(function (j) { release = j; renderRelease(); })
    .catch(function () { releaseFailed = true; renderRelease(); });

  fetch(API, { headers: { Accept: 'application/vnd.github+json' } })
    .then(function (r) { return r.ok ? r.json() : null; })
    .then(function (j) { if (j && typeof j.stargazers_count === 'number') document.getElementById('stars').textContent = j.stargazers_count; })
    .catch(function () {});

  // ---- contribuidores ----
  var contributors = null, contribFailed = false;
  function renderContributors() {
    var box = document.getElementById('contributors');
    if (!contributors) { if (contribFailed) box.innerHTML = '<span class="empty">' + t('c_fail') + '</span>'; return; }
    box.innerHTML = '';
    contributors.forEach(function (c) {
      var a = document.createElement('a'); a.className = 'person'; a.href = c.html_url; a.target = '_blank'; a.rel = 'noopener';
      var img = document.createElement('img'); img.src = c.avatar_url + '&s=80'; img.alt = '';
      var d = document.createElement('div'); d.innerHTML = '<b></b><small></small>';
      d.querySelector('b').textContent = c.login; d.querySelector('small').textContent = c.contributions + ' ' + t('c_commits');
      a.appendChild(img); a.appendChild(d); box.appendChild(a);
    });
  }
  fetch(API + '/contributors?per_page=100', { headers: { Accept: 'application/vnd.github+json' } })
    .then(function (r) { if (!r.ok) throw new Error(r.status); return r.json(); })
    .then(function (j) { contributors = Array.isArray(j) ? j.filter(function (c) { return c.type !== 'Bot'; }) : []; renderContributors(); })
    .catch(function () { contribFailed = true; renderContributors(); });

  // ---- apoiantes (donors.json) ----
  var donors = null;
  function renderDonors() {
    var box = document.getElementById('donors');
    if (!donors) return;
    if (!donors.length) { box.innerHTML = '<span class="empty">' + t('g_empty') + '</span>'; return; }
    box.innerHTML = '';
    donors.slice().reverse().forEach(function (d) {
      var el = document.createElement('div'); el.className = 'donor';
      var b = document.createElement('b'); b.textContent = d.name || 'Anónimo';
      var s = document.createElement('span'); s.textContent = [d.country, d.date, d.amount].filter(Boolean).join(' · ');
      el.appendChild(b); el.appendChild(s);
      if (d.message) { var em = document.createElement('em'); em.textContent = '“' + d.message + '”'; el.appendChild(em); }
      box.appendChild(el);
    });
    var th = document.createElement('p'); th.className = 'meta'; th.textContent = t('g_thanks'); th.style.gridColumn = '1 / -1'; box.appendChild(th);
  }
  fetch('donors.json?' + Date.now()).then(function (r) { return r.ok ? r.json() : { donors: [] }; })
    .then(function (j) { donors = (j && j.donors) || []; renderDonors(); })
    .catch(function () { donors = []; renderDonors(); });

  // ---- galeria ----
  var lb = document.getElementById('lightbox');
  document.querySelectorAll('.shot-zoom').forEach(function (img) {
    img.addEventListener('click', function () { lb.querySelector('img').src = img.src; lb.classList.add('on'); });
  });
  lb.addEventListener('click', function () { lb.classList.remove('on'); });
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape') lb.classList.remove('on'); });

  // ---- menu e idioma ----
  var menu = document.getElementById('menu');
  document.getElementById('menuToggle').addEventListener('click', function () { menu.classList.toggle('open'); });
  menu.addEventListener('click', function () { menu.classList.remove('open'); });
  document.querySelectorAll('#lang button').forEach(function (b) { b.addEventListener('click', function () { applyLang(b.getAttribute('data-lang')); }); });
  document.getElementById('year').textContent = new Date().getFullYear();

  var saved = null;
  try { saved = localStorage.getItem('sp-lang'); } catch (e) {}
  var nav = (navigator.language || 'pt').slice(0, 2).toLowerCase();
  applyLang(saved || (T[nav] ? nav : 'en'));
})();
