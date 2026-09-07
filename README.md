# macread — ler e copiar arquivos de discos de Mac e Linux no Windows

[![build](https://github.com/arturjose0/macread/actions/workflows/build.yml/badge.svg)](https://github.com/arturjose0/macread/actions)
[![release](https://img.shields.io/github/v/release/arturjose0/macread)](https://github.com/arturjose0/macread/releases/latest)
[![license](https://img.shields.io/badge/licen%C3%A7a-MIT-blue.svg)](LICENSE)

> **English:** macread reads Mac (APFS, HFS+) and Linux (ext2/3/4, LVM) disks and DMG images
> directly on Windows, without installing drivers, and copies files out of them. It can also
> mount a volume as a read-only Windows drive letter through Dokan. Download the executables
> from the [Releases page](https://github.com/arturjose0/macread/releases/latest); the rest of
> this document is in Portuguese. Contributions are welcome, see [CONTRIBUTING.md](CONTRIBUTING.md).

## Baixar

Vá à página de [**Releases**](https://github.com/arturjose0/macread/releases/latest) e baixe
`macread-windows-x64.zip` (contém `macread-gui.exe`, `macread.exe` e este manual). Não há
instalação: descompacte numa pasta e execute. Windows 10/11 de 64 bits.

`macread` lê **diretamente** o conteúdo de discos formatados pelo macOS (**APFS** e
**HFS+/HFSX**) e pelo Linux (**ext2/ext3/ext4**, inclusive dentro de volumes **LVM**) a partir
do Windows, sem instalar driver nenhum. Serve para discos internos retirados de um
MacBook/iMac ou de um PC com Linux e ligados por adaptador USB, discos externos, imagens
brutas (`dd`, `.img`, `.raw`) e imagens **DMG** do Utilitário de Disco.

É somente leitura: nunca grava no disco de origem.

Dois programas prontos, sem dependências:

* `macread-gui.exe`: **interface gráfica**. Dê dois cliques, aceite o pedido de permissão de
  Administrador, e navegue pelos discos como no Explorador de Arquivos.
* `macread.exe`: linha de comando (para scripts ou cópias grandes sem supervisão). Para ler
  um disco físico use [`abrir-como-admin.cmd`](abrir-como-admin.cmd), que abre um Prompt como
  Administrador já na pasta e lista os discos.

## Interface gráfica (macread-gui.exe)

* À esquerda, a árvore mostra cada disco físico com suas partições (e, no APFS, os volumes;
  no LVM, os volumes lógicos). Clique numa partição marcada com `◄ Mac` ou `◄ Linux`.
* À direita aparece o conteúdo da pasta: dê dois cliques numa pasta para entrar, use
  **▲ Acima** para voltar; dois cliques num arquivo mostram tamanho, datas e permissões.
* **Copiar selecionados para...** copia os itens marcados na lista (Ctrl/Shift para vários);
  **Copiar pasta atual para...** copia a pasta inteira; **Verificar leitura** lê tudo sem
  gravar, para conferir se o disco está íntegro. O progresso aparece na barra inferior e
  **Cancelar** interrompe a qualquer momento. Ao terminar, um resumo informa quantos
  arquivos foram copiados e os erros, que ficam também em `macread-log.txt` no destino.
* **Montar como unidade** transforma o volume selecionado numa letra de unidade do
  Windows (por exemplo `Z:`), somente leitura, visível no Explorador e em qualquer programa
  (abrir fotos, PDFs, vídeos direto do disco, copiar com o Explorador…). O Explorador abre
  sozinho na nova unidade. Clique em **Desmontar** (o mesmo botão) antes de remover o disco
  ou fechar o programa; fechar o programa também desmonta. Veja "Montar como unidade" abaixo.
* **Abrir imagem/DMG...** adiciona um arquivo de imagem (`.img`, `.raw`, `.dd`, `.dmg`) à
  árvore. Também é possível arrastar a imagem sobre o `macread-gui.exe`.
* Sem privilégios de Administrador só as imagens funcionam; o programa pede a elevação
  sozinho ao abrir (`--sem-admin` desliga esse pedido).

### Montar como unidade (letra de disco)

Uma letra de unidade de verdade exige um driver de sistema de arquivos em modo usuário. O
macread usa o **Dokan 2** (gratuito, código aberto, <https://github.com/dokan-dev/dokany>;
alguns programas, como o Paragon, já o instalam). Se não estiver instalado, baixe o
`DokanSetup.exe` da página de releases do Dokan, instale e reinicie; o botão passa a
funcionar. Sem o Dokan, o programa avisa e todas as outras funções continuam disponíveis.

* A unidade é **somente leitura**: nada é gravado no disco de origem; tentativas de gravar
  recebem "acesso negado".
* Nomes inválidos no Windows são ajustados (`:` vira `_`, etc.); links simbólicos e arquivos
  especiais não aparecem na unidade (use a lista da interface para vê-los).
* Pela linha de comando: `macread montar disco:1 X:` (Ctrl+C desmonta) e
  `macread desmontar X`.
* Se uma unidade ficar "presa" (programa fechado à força), `macread desmontar X` resolve.

### Precisa de senha?

Não, para discos comuns. A senha de login do Linux ou do macOS só protege a sessão do
sistema; os arquivos ficam gravados em texto claro no disco, e as permissões (`drwx------`)
são aplicadas apenas pelo sistema que está rodando. Lendo o disco diretamente, como o
macread faz, tudo é acessível sem senha. O disco Ubuntu testado é assim: `/home/<usuário>`
abriu normalmente.

A senha faria diferença apenas se o disco tivesse sido **criptografado de propósito**:
FileVault no Mac, LUKS ("criptografar a nova instalação" no Ubuntu) ou pastas pessoais
eCryptfs (Ubuntu antigo, reconhecível pela pasta `.ecryptfs`). Nesses casos o macread mostra
a partição como criptografada e ainda não consegue abri-la, mesmo com a senha.

## O que ela faz

| Recurso | Suporte |
|---|---|
| APFS (macOS 10.13+), inclusive volumes com várias versões de metadados (checkpoints) | ✔ |
| Vários volumes no mesmo container APFS (Sistema, Dados, Preboot…) | ✔ (escolha com `-v`) |
| HFS+ e HFSX (macOS até 10.12, Time Machine antigo, discos externos "Mac OS Extended") | ✔ |
| HFS+ embutido em HFS clássico (wrapper) e Apple Partition Map (Macs PowerPC) | ✔ |
| **ext4, ext3, ext2** (Ubuntu, Debian, Mint, etc.): extents, blocos indiretos, `inline_data`, `64bit`, `metadata_csum`, arquivos esparsos, pastas grandes (htree) | ✔ |
| **LVM2**: volumes lógicos lineares (instalações "usar LVM" do Ubuntu, Fedora, RHEL) | ✔ (aparecem como partições extras) |
| Tabelas GPT (inclusive com cabeçalho de backup), MBR híbrido e imagens sem partição | ✔ |
| Imagens DMG (UDIF: zlib, bzip2, ADC, LZFSE, LZMA, blocos zero/brutos) | ✔ leitura direta, sem converter |
| Arquivos comprimidos pelo macOS (decmpfs): zlib, LZVN, LZFSE, sem compressão | ✔ (descomprime na hora) |
| Hard links de arquivos e pastas do HFS+ (backups do Time Machine) | ✔ |
| Links simbólicos | listados; opcionalmente gravados como `.symlink.txt` |
| Volumes APFS "selados" (System de macOS 11+) | ✔ |
| Nomes longos, nomes com caracteres inválidos no Windows, caminhos > 260 caracteres | ✔ (sanitiza e usa `\\?\`) |
| Retomada: arquivo já existente no destino com o mesmo tamanho é pulado | ✔ |
| Datas de modificação preservadas | ✔ |
| Verificação de leitura (`verificar`): lê tudo sem gravar e relata erros | ✔ |
| XFS (RHEL/CentOS), Btrfs (Fedora, openSUSE), F2FS | ✘ detectados, ainda não lidos |
| LUKS (Linux criptografado), fscrypt (pastas criptografadas do ext4) | ✘ precisam da senha/chave |
| LVM com thin pool, RAID, espelho ou volumes espalhados por vários discos | ✘ só o mapeamento linear num disco |
| FileVault (volume APFS criptografado) | ✘ precisa da senha; não suportado |
| Core Storage (FileVault 2 antigo, Fusion Drive) | ✘ |
| Snapshots APFS (backups do Time Machine em APFS) | ✘ só o estado atual do volume |
| Macs com chip T2 ou Apple Silicon (SSD soldado e sempre criptografado) | ✘ o disco não é legível fora do Mac |
| HFS clássico (anterior a 1998), LZBITMAP, DMG criptografado ou sparsebundle | ✘ |

## Linha de comando (macread.exe)

Abra o **PowerShell ou Prompt de Comando como Administrador** (obrigatório para ler um
disco físico; para imagens em arquivo não precisa).

```text
macread discos
```
Lista os discos físicos com suas partições; as partições legíveis aparecem marcadas com
`◄ Mac` ou `◄ Linux`. Volumes lógicos LVM aparecem como partições adicionais.

```text
macread info disco:1
macread ls disco:1 /Users
macread ls disco:1 /home/joao -l
macread arvore disco:1 /home --nivel 2
macread copiar disco:1 /Users/joao/Documents D:\Recuperado
macread copiar disco:1 /home/joao D:\Recuperado
macread copiar disco:1 / D:\Recuperado\DiscoInteiro
macread verificar disco:1 /home/joao
```

`disco:1` é o número mostrado por `macread discos` (o mesmo do Gerenciamento de Disco do
Windows). Também aceita `\\.\PhysicalDrive1`, o caminho de uma imagem bruta (`C:\disco.img`)
ou de um DMG (`C:\backup.dmg`).

Ao copiar uma pasta, ela é criada dentro do destino (`D:\Recuperado\joao`). Ao copiar
a raiz `/`, o conteúdo vai direto para o destino. Em nomes com acentos no Prompt de
Comando, rode antes `chcp 65001` (ou use a interface gráfica, que não tem esse problema).

### Comandos

| Comando | O que faz |
|---|---|
| `discos` | lista os discos físicos e suas partições |
| `info <origem>` | partições, container APFS, volumes LVM e detalhes do sistema de arquivos |
| `ls <origem> [caminho] [-l]` | lista uma pasta |
| `arvore <origem> [caminho] [--nivel N]` | mostra a árvore de pastas |
| `cat <origem> <arquivo>` | envia um arquivo para a saída padrão |
| `copiar <origem> <caminho> <destino>` | copia um arquivo ou pasta (recursivo) |
| `verificar <origem> [caminho]` | lê todos os arquivos sem gravar nada (testa a leitura) |
| `hex <origem> [offset] [tamanho]` | mostra bytes brutos do disco (diagnóstico) |
| `montar <origem> [letra]` | monta como unidade do Windows via Dokan (Ctrl+C desmonta) |
| `desmontar <letra>` | desmonta uma unidade montada pelo macread |

### Opções

| Opção | Efeito |
|---|---|
| `-p N`, `--particao N` | usa a partição N (padrão: primeira partição Mac; se não houver, a primeira Linux) |
| `-v N`, `--volume N` | usa o volume APFS N (padrão: o volume "Dados", onde ficam os arquivos do usuário) |
| `-l` | listagem detalhada (permissões, tamanho em bytes, data, `C` = comprimido) |
| `--sobrescrever` | regrava arquivos que já existem no destino |
| `--links` | grava um `nome.symlink.txt` com o destino de cada link simbólico |
| `--verboso` | mostra cada arquivo copiado |
| `--simular` | só percorre e conta, não grava nada |
| `--forcar` | tenta abrir um volume marcado como criptografado |
| `--nivel N` | profundidade do comando `arvore` (padrão 3) |

Erros durante a cópia (arquivo corrompido, bloco ilegível…) não interrompem o processo:
são mostrados na tela e gravados em `macread-log.txt` na pasta de destino. Rodar o mesmo
comando de novo continua de onde parou, pulando o que já foi copiado.

### Onde estão os arquivos do usuário?

* macOS 10.15 ou mais novo (APFS): volume **"Macintosh HD - Data"** (função *Dados*),
  pasta `/Users/<nome>`. Esse volume é escolhido automaticamente.
* macOS 10.13–10.14 (APFS) ou HFS+: volume único, pasta `/Users/<nome>`.
* Backups do Time Machine em HFS+: `/Backups.backupdb/<Mac>/<data>/...`.
* Linux: `/home/<nome>`. Em instalações com LVM, escolha o volume lógico "root" (ou o
  que contém `/home`); `macread info` e a interface gráfica mostram qual é qual. Se houver
  uma partição `/home` separada, ela aparece como outra partição ext4.

## Compilar

Requisitos: [Rust](https://rustup.rs) (1.80 ou mais novo). Não é preciso o Visual Studio se
usar a toolchain GNU, que traz o próprio linker (com o Visual Studio Build Tools instalado a
toolchain MSVC padrão também serve, e é a usada pelo GitHub Actions). Basta rodar
[`build.cmd`](build.cmd) ou:

```text
rustup toolchain install stable-x86_64-pc-windows-gnu
cargo +stable-x86_64-pc-windows-gnu build --release
```

Os executáveis ficam em `target\release\macread.exe` e `target\release\macread-gui.exe`.
Se o Visual Studio Build Tools estiver instalado, `cargo build --release` com a toolchain
MSVC padrão também funciona. A interface usa a biblioteca `native-windows-gui` (controles
nativos do Windows, sem runtime extra).

Testes unitários (decodificador LZVN validado contra o codificador LZFSE de referência,
containers decmpfs sintéticos, metadados LVM, nomes, datas):

```text
cargo +stable-x86_64-pc-windows-gnu test
```

## Como foi validado

Conteúdo comparado byte a byte com o extraído pelo 7-Zip 26 (que também lê APFS, HFS+ e ext):

* imagens de teste do dfvfs/plaso: `apfs.raw`, `apfs.dmg`, `hfsplus.raw`, `hfsplus_zlib.dmg`
  (HFS+ dentro de DMG zlib) e `apm.dmg` (Apple Partition Map);
* `wsdf.dmg` (APFS, projeto *afro*): 18/18 arquivos idênticos;
* `BaseSystem.dmg` do macOS Monterey (HFS+ com 45.670 arquivos dentro de DMG zlib) e do
  High Sierra (35.473 arquivos): todos os arquivos lidos sem erro, amostras idênticas ao 7-Zip;
* DMG do Safari Technology Preview (HFSX dentro de DMG LZFSE): pacote de 219 MB idêntico;
* imagens ext4 (4 KB e 1 KB, `inline_data`, `64bit`, `metadata_csum`), ext3 e ext2 criadas com
  `mke2fs -d` (3.013 arquivos cada, com pasta de 3.000 entradas, arquivo esparso de 40 MB,
  links longos e nomes Unicode): tudo idêntico; nos casos em que o 7-Zip não consegue ler
  (`inline_data`, arquivo esparso em ext2/ext3) o resultado foi conferido entre as imagens;
* imagem LVM2 sintética com dois volumes lógicos (um deles com dois segmentos fora de ordem);
* discos físicos via `\\.\PhysicalDrive` com privilégios de Administrador: SSD Windows (GPT com
  10 partições) e disco USB Ubuntu (EFI + ext4: `/etc` e `/usr/share`, 83.399 arquivos, lidos
  sem erro);
* interface gráfica testada por automação de teclado (abrir volume, navegar, copiar);
* montagem Dokan: imagem ext4 montada como `W:`, listagem, pasta com 3.000 arquivos,
  caminhos profundos, nomes Unicode e hashes SHA-1 de arquivos de 20 MB, 40 MB (esparso) e
  4 KB idênticos aos originais lidos pelo Windows.

Ainda **não** foi testado com um disco real que contenha arquivos comprimidos pelo macOS
(decmpfs) nem com metadados LVM gravados pelo próprio `lvm`; essas partes seguem a
especificação e passam nos testes sintéticos. Se algum arquivo não abrir, use
**Verificar leitura** (ou `macread verificar`) e guarde a saída.

## Como funciona

1. Abre `\\.\PhysicalDriveN` com leituras alinhadas ao setor (ou um arquivo de imagem; DMGs
   são descomprimidos bloco a bloco sob demanda) e interpreta a tabela de partições (GPT,
   MBR, APM). Identifica o sistema de arquivos pelas assinaturas (`NXSB` para APFS,
   `H+`/`HX` para HFS+, `0xEF53` para ext, `LABELONE` para LVM…). Partições LVM são
   expandidas lendo os metadados em texto do grupo de volumes.
2. **APFS**: localiza o checkpoint mais recente com checksum Fletcher-64 válido, percorre o
   object map do container, lê os superblocos dos volumes, o object map do volume e a
   árvore B do sistema de arquivos (inodes, entradas de pasta, atributos estendidos,
   extents). Arquivos com `com.apple.decmpfs` são descomprimidos de forma transparente.
3. **HFS+**: lê o cabeçalho do volume e as árvores B de catálogo, extents e atributos,
   resolvendo extents adicionais e hard links.
4. **ext2/3/4**: lê o superbloco, os descritores de grupo (inclusive `meta_bg`), os inodes,
   as árvores de extents ou os blocos indiretos, dados inline e as entradas de pasta.
5. A cópia lê em blocos de 1 MB, grava com nome sanitizado para o Windows e ajusta a data
   de modificação. Na interface gráfica a cópia roda numa thread separada, com progresso
   e cancelamento.
6. **Montagem**: o volume é aberto numa thread própria (`fsservice.rs`) que atende pedidos de
   listagem/leitura por mensagens; o módulo `dokan.rs` carrega a `dokan2.dll` em tempo de
   execução e implementa as callbacks do Dokan (criar/abrir, ler, informações, listar pasta,
   informações do volume) sobre esse serviço.

Bibliotecas usadas: `flate2` (zlib), `lzfse_rust` (LZFSE), `bzip2-rs`, `lzma-rs`,
`unicode-normalization` (comparação de nomes NFD), `native-windows-gui` (interface),
`winapi` (chamadas do Windows). Decodificadores LZVN e ADC próprios; ligação ao Dokan feita
à mão (sem SDK) a partir do `dokan.h` 2.3.1.

## Contribuir

O projeto é aberto (licença [MIT](LICENSE)) e aceita *issues* e *pull requests*: relatos de
teste com discos reais, novos sistemas de arquivos (XFS, Btrfs), suporte a LUKS/FileVault com
senha, traduções. Veja [CONTRIBUTING.md](CONTRIBUTING.md). Cada *push* e cada *tag* `v*` é
compilado e testado automaticamente pelo GitHub Actions, que também publica os executáveis
na release.

## Limitações e avisos

* Se o Mac tinha **FileVault ativado**, ou o Linux usa **LUKS**, os dados estão criptografados
  com a senha do usuário e esta ferramenta não os abre.
* MacBooks a partir de 2018 (chip T2) e todos os Apple Silicon têm o SSD soldado e
  criptografado por hardware: não há como ler fora do próprio Mac.
* Um volume ext4 que não foi desmontado corretamente pode ter um journal pendente; a
  ferramenta avisa e lê o estado gravado no disco, que pode não incluir os últimos segundos
  de gravação.
* Se o disco tiver setores defeituosos, os arquivos afetados aparecem no log de erros e o
  resto continua sendo copiado. Para discos em mau estado, faça antes uma imagem com uma
  ferramenta de clonagem e aponte o `macread` para a imagem.
* Resource forks, atributos estendidos e permissões Unix não são copiados; apenas o
  conteúdo dos arquivos e a data de modificação.
* Nomes que só diferem por maiúsculas/minúsculas (comum no Linux) ou por caracteres
  proibidos no Windows (`a:b` e `a_b`) acabam no mesmo arquivo de destino; o segundo é
  pulado ou sobrescreve o primeiro.
