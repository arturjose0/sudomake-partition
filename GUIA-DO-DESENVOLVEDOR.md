# Guia do desenvolvedor — SUDOMAKE Partition

Este guia é para quem vai mexer no código: como executar o projecto, onde está cada coisa,
como editar, onde ficam os executáveis, como enviar para o GitHub e como criar o instalador.
Para o manual de utilização veja o [README.md](README.md).

## 1. O que precisa ter instalado

| Programa | Para quê | Onde obter |
|---|---|---|
| **Rust** (rustup, cargo) | compilar o projecto | <https://rustup.rs> — instale e depois corra `rustup toolchain install stable-x86_64-pc-windows-gnu` |
| **Git** | controlar versões e enviar para o GitHub | <https://git-scm.com/download/win> (ou o GitHub Desktop) |
| **Inno Setup 6** | criar o instalador `.exe` | <https://jrsoftware.org/isdl.php> |
| **Dokan 2** (opcional) | testar "Montar como unidade" | <https://github.com/dokan-dev/dokany/releases> (`DokanSetup.exe`) |
| **VS Code** + extensão *rust-analyzer* (opcional) | editar com ajuda e erros em tempo real | <https://code.visualstudio.com> |

Não precisa do Visual Studio: a toolchain **GNU** do Rust traz o próprio linker. Se tiver o
Visual Studio Build Tools, a toolchain MSVC normal também funciona.

## 2. Abrir o projecto

O projecto está em `C:\laragon\www\apple\macread` (a pasta manteve o nome antigo; o programa
chama-se SUDOMAKE Partition). Noutro computador basta clonar:

```text
git clone https://github.com/arturjose0/sudomake-partition.git
cd sudomake-partition
```

Abra a pasta no VS Code (`code .`) ou edite os ficheiros com qualquer editor.

## 3. Compilar e executar

A forma mais simples é dar dois cliques em **`build.cmd`**: ele instala a toolchain GNU se
faltar, compila em modo *release* e copia os executáveis para a raiz do projecto.

Na linha de comando (PowerShell ou Prompt, dentro da pasta do projecto):

```text
cargo +stable-x86_64-pc-windows-gnu build --release        # compila tudo (interface + linha de comando)
cargo +stable-x86_64-pc-windows-gnu test                   # corre os testes unitários
cargo +stable-x86_64-pc-windows-gnu run --release --bin SudomakePartition        # compila e abre a interface
cargo +stable-x86_64-pc-windows-gnu run --release --bin sudomake-partition -- discos   # linha de comando
```

Dica: se correr `rustup default stable-x86_64-pc-windows-gnu` uma vez, pode escrever só
`cargo build --release`, sem o `+stable-x86_64-pc-windows-gnu`.

A primeira compilação demora alguns minutos (descarrega e compila as bibliotecas); as
seguintes demoram segundos. Só o que mudou é recompilado.

### Onde ficam os executáveis

| Ficheiro | O que é |
|---|---|
| `target\release\SudomakePartition.exe` | interface gráfica (o programa principal) |
| `target\release\sudomake-partition.exe` | linha de comando |
| `target\debug\...` | os mesmos, em modo *debug* (mais lentos, com mais verificações), se compilar sem `--release` |
| `SudomakePartition.exe` e `sudomake-partition.exe` na raiz | cópias feitas pelo `build.cmd` |
| `dist\sudomake-partition-setup-1.3.1.exe` | o instalador, depois de correr o Inno Setup (secção 7) |

A pasta `target\` pode ficar com mais de 1 GB; pode apagá-la sem problema (`cargo clean`),
tudo é recriado na próxima compilação. `target\`, `dist\`, `*.exe` e `*.zip` estão no
`.gitignore`: nunca vão para o GitHub — os executáveis só são publicados nas *releases*.

### Executar e testar

* A interface pede permissão de **Administrador** ao abrir (é obrigatório para ler discos
  físicos). Para testar sem isso, use imagens de disco:
  `target\release\SudomakePartition.exe --sem-admin C:\caminho\imagem.dmg`.
* `--lang=en` (ou `pt`, `fr`, `es`) abre já noutro idioma. O idioma e o tema escolhidos ficam
  guardados em `%APPDATA%\SUDOMAKE\Partition\config.txt` (apague o ficheiro para voltar ao padrão).
* Se um erro interno acontecer, aparece uma mensagem e o texto fica em
  `%TEMP%\sudomake-partition-panic.txt`.
* Se a compilação disser **"Acesso negado"** ao gravar o `.exe`, é porque o programa ainda está
  aberto: feche-o e compile de novo.

## 4. Ficheiros principais

```text
macread\
├─ Cargo.toml                  nome, versão e bibliotecas do projecto
├─ src\
│  ├─ i18n.rs                  TODOS os textos da interface (PT, EN, FR, ES) e os dados de contacto/doação
│  ├─ bin\gui.rs               interface gráfica (janela, azulejos, temas, botões)
│  ├─ main.rs                  linha de comando (comandos discos, info, ls, copiar, montar…)
│  ├─ lib.rs                   lista dos módulos da biblioteca
│  ├─ device.rs                acesso aos discos físicos e a imagens de disco
│  ├─ partition.rs             tabelas de partições (GPT, MBR, APM) e identificação do sistema de ficheiros
│  ├─ apfs.rs / hfsplus.rs     leitores de discos Mac
│  ├─ ext4.rs / lvm.rs         leitores de discos Linux
│  ├─ dmg.rs                   imagens DMG
│  ├─ decmpfs.rs / lzvn.rs     ficheiros comprimidos do macOS
│  ├─ osdetect.rs              descobre qual sistema está em cada partição (Ubuntu, macOS, Windows…)
│  ├─ fs.rs                    interface comum a todos os leitores
│  ├─ open.rs                  abre uma origem e escolhe partição/volume
│  ├─ copy.rs                  cópia de ficheiros com progresso, registo e retomada
│  └─ fsservice.rs / dokan.rs  montagem como unidade do Windows (driver Dokan)
├─ installer\
│  ├─ sudomake-partition.iss   script do instalador (Inno Setup)
│  ├─ sobre-pt.txt, sobre-en.txt, sobre-fr.txt, sobre-es.txt   texto "Informação" do instalador
│  └─ sudomake-partition.ico   ícone do programa e do instalador
├─ .github\workflows\build.yml  compilação automática no GitHub (Actions) e publicação das releases
├─ docs\                       site do projecto (GitHub Pages: index.html, site.css, site.js, donors.json, img\) e capturas usadas no README
├─ README.md                   manual público (é o que aparece no GitHub)
├─ CONTRIBUTING.md             regras para quem contribui
├─ build.cmd                   compila e copia os executáveis
└─ abrir-como-admin.cmd        abre um Prompt como Administrador na pasta do programa
```

## 5. Como editar as coisas mais comuns

Depois de qualquer alteração: `cargo build --release` e abra o `.exe` para ver o resultado.

**Mudar telefone, e-mail, site, YouTube, GitHub, PayPal, nome da empresa**
→ `src/i18n.rs`, constantes no início do ficheiro (`PHONE_DISPLAY`, `EMAIL`, `SITE`, `YOUTUBE`…).
Todos os botões e páginas usam essas constantes. O instalador tem os mesmos dados no topo de
`installer/sudomake-partition.iss` (`#define Email`, `#define PhoneLocal`…) e nos ficheiros
`installer/sobre-*.txt`; o README também os repete.

**Mudar um texto da interface** (botão, mensagem, página Sobre/Doar)
→ `src/i18n.rs`, dentro de `table! { ... }`. Cada linha é
`chave => ["português", "english", "français", "español"]`. Mude os quatro. `{0}`, `{1}` são
valores preenchidos pelo programa (não apague). `\r\n` é uma quebra de linha.

**Acrescentar um idioma** → `src/i18n.rs`: acrescente a variante em `enum Lang`, em `ALL`,
`code()`, `name()`, `from_code()`, `from_langid()` (código do Windows), uma coluna a mais em
cada linha de `table!` e na macro. No instalador, acrescente a língua em `[Languages]` e as
mensagens em `[Messages]`/`[CustomMessages]`.

**Cores dos temas claro/escuro** → `src/bin/gui.rs`, função `palette(dark)`: fundo, azulejo,
azulejo seleccionado, borda, texto, barra de espaço. `rgb(r, g, b)`.

**Tamanho dos azulejos e da janela** → `src/bin/gui.rs`: `TILE_W`, `TILE_H`, `GAP` (azulejos)
e `.size((1400, 800))` na função `build` (janela inicial).

**Que botões aparecem em cada situação** → `src/bin/gui.rs`, função `update_toolbar` (a
barra é contextual: só mostra o que faz sentido para o disco, pasta ou ficheiro seleccionado).
**Posição dos botões** → função `layout`. **Desenho dos azulejos** (incluindo a etiqueta "Z:"
das unidades montadas) → função `paint_tiles`. **O que aparece em cada azulejo** (título,
sistema, espaço livre) → função `describe_source`, que corre numa thread separada
(`scan_sections`) para a janela nunca bloquear. **Actualização automática** → temporizadores
`TIMER_PERIODIC` (30 s) e `TIMER_DEVICE` (2,5 s depois de um aviso `WM_DEVICECHANGE`).
**Cópia de disco completo e verificação de espaço** → função `copy_disk` (usa
`disk_free_space` e `FileSystem::used`).

**Comandos da linha de comando** → `src/main.rs` (`usage()` é o texto de ajuda; o `match` em
`main()` liga cada comando à sua função).

**Detecção do sistema** (por exemplo reconhecer outra distribuição) → `src/osdetect.rs`,
função `detect` (procura `/etc/os-release`, `SystemVersion.plist`, `/home`, `/Users`…) e
`classify_partition` (EFI, Windows, swap…).

**Ícone / logótipo** → a origem é `imagens/favicon.png`. O script gera
`installer/sudomake-partition.ico` (tamanhos 16 a 256), `installer/wizard-small.bmp` (imagem
do assistente do instalador) e `docs/logo.png` (README). O `build.rs` embute o `.ico` nos dois
executáveis (ícone da janela, da barra de tarefas e do Explorador) sem precisar de `rc.exe`
nem `windres`; o ecrã inicial desenha o mesmo ícone no canto superior direito. Para mudar o
logótipo basta substituir o PNG e voltar a gerar os ficheiros.

**Versão nova** → mude o número em **dois** sítios: `Cargo.toml` (`version = "1.3.1"`) e
`installer/sudomake-partition.iss` (`#define AppVersion "1.3.1"`). A interface, a linha de
comando e o instalador lêem daí.

## 6. Enviar para o GitHub

O repositório é <https://github.com/arturjose0/sudomake-partition>. Só precisa configurar o
Git uma vez (já está feito neste computador):

```text
git config --global user.name "José Artur Kassala"
git config --global user.email "52246841+arturjose0@users.noreply.github.com"
```

### Enviar alterações do dia-a-dia

```text
git status                       # mostra o que mudou
git add -A                       # marca tudo para o commit
git commit -m "Descrição curta do que mudou"
git push                         # envia para o GitHub
```

Cada `git push` para o ramo `main` faz o GitHub Actions compilar e testar o projecto
(separador **Actions** no site). Se aparecer um ✗ vermelho, clique nele para ver o erro.

Escreva os commits só em seu nome; não acrescente linhas `Co-Authored-By` de outras
ferramentas, senão elas aparecem como contribuidoras do projecto.

Se preferir uma janela em vez de comandos: o **GitHub Desktop** faz o mesmo (Changes →
Summary → *Commit to main* → *Push origin*).

### Publicar uma versão nova (release com executáveis)

1. Mude a versão em `Cargo.toml` e em `installer/sudomake-partition.iss` (secção 5).
2. Compile e teste localmente: `cargo build --release` e `cargo test`.
3. Envie e crie a *tag* com o número da versão, precedido de `v`:

   ```text
   git add -A
   git commit -m "SUDOMAKE Partition 1.3.1: o que mudou"
   git push
   git tag v1.3.1
   git push origin v1.3.1
   ```

4. A *tag* `v*` faz o GitHub Actions compilar, testar, criar o instalador e publicar
   automaticamente a release com quatro ficheiros: `sudomake-partition-setup-1.3.1.exe`,
   `sudomake-partition-windows-x64.zip`, `sudomake-partition.exe` e `SudomakePartition.exe`.
   Demora uns 3 a 5 minutos.
5. Abra <https://github.com/arturjose0/sudomake-partition/releases>, clique no lápis da
   release nova e escreva as novidades (em português e uma linha em inglês). Marque
   *Set as the latest release*.

Se se enganou na *tag*: `git tag -d v1.3.1` apaga localmente e
`git push origin :refs/tags/v1.3.1` apaga no GitHub; depois crie de novo.

## 7. Criar o instalador

O instalador é feito pelo **Inno Setup 6** a partir de `installer/sudomake-partition.iss`.
Ele empacota os dois `.exe` de `target\release`, por isso compile primeiro.

**Pelo programa:** abra o Inno Setup Compiler, *File → Open* →
`installer\sudomake-partition.iss`, e carregue em **Compile** (Ctrl+F9).

**Pela linha de comando:**

```text
cargo +stable-x86_64-pc-windows-gnu build --release
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer\sudomake-partition.iss
```

O resultado é `dist\sudomake-partition-setup-<versão>.exe`. Para testar sem cliques:

```text
dist\sudomake-partition-setup-1.3.1.exe /VERYSILENT /CURRENTUSER /NORESTART
```

instala em `%LOCALAPPDATA%\Programs\SUDOMAKE Partition`; para desinstalar,
`unins000.exe /VERYSILENT` nessa pasta.

O que o script faz e onde mudar:

* `[Setup]` — nome, versão, pasta, ícone, se pergunta "só para mim / todos os utilizadores".
* `[Languages]` — os idiomas do instalador e o texto "Informação" (`sobre-*.txt`).
* `[Messages]` e `[CustomMessages]` — textos da página de boas-vindas, das tarefas, da página
  de doação e das mensagens de erro, nos cinco idiomas.
* `[Files]` — os ficheiros copiados; `[Icons]` — atalhos do menu Iniciar e do ambiente de
  trabalho; `[Run]` — o que abrir no fim.
* `[Code]` — a verificação do Dokan (`DokanInstalled`), o download e instalação do `.msi`
  (`PrepareToInstall`), o PATH e a **página de doação** com ligações (`CreateDonatePage`).
* Para actualizar o Dokan: mude `DokanVersion`, `DokanUrl` e `DokanSha256` no topo. O SHA-256
  obtém-se com `certutil -hashfile Dokan_x64.msi SHA256` depois de descarregar o ficheiro.

No GitHub Actions o instalador é criado da mesma forma (o `build.yml` instala o Inno Setup
com `choco` e corre o `ISCC.exe`), por isso o que funciona aqui funciona lá.

## 8. Site do projecto (GitHub Pages)

O site <https://arturjose0.github.io/sudomake-partition/> é servido directamente da pasta
`docs\` do ramo `main`: qualquer `git push` que mude ficheiros em `docs\` actualiza o site em
um ou dois minutos (o GitHub mostra o progresso no separador *Actions*, "pages build and
deployment").

* `docs\index.html` — a página (textos em português no HTML; os outros idiomas estão no
  dicionário `T` de `docs\site.js`, que também obtém em tempo real da API do GitHub a última
  release, o número de estrelas e a lista de contribuidores).
* `docs\donors.json` — a lista de apoiantes. Para acrescentar um, edite o ficheiro e adicione
  um objecto à lista `donors`: `{"name": "Nome ou Anónimo", "country": "Angola", "date":
  "2026-09-09", "amount": "2000 Kz", "message": "opcional"}`. Peça sempre autorização antes de
  publicar um nome. Faça `git add`, `git commit`, `git push` e o site actualiza.
* `docs\img\` — as capturas de ecrã. Para renovar, tire capturas novas do programa (1400×800)
  e substitua os ficheiros com o mesmo nome.
* `docs\logo.png` — o logótipo usado no site, no README e no favicon.

## 9. Problemas comuns

| Sintoma | Causa e solução |
|---|---|
| `error: linker link.exe not found` | está a usar a toolchain MSVC sem Visual Studio. Use `cargo +stable-x86_64-pc-windows-gnu ...` ou `rustup default stable-x86_64-pc-windows-gnu`. |
| `Acesso negado (os error 5)` ao compilar | o `.exe` está aberto. Feche o programa (ou termine-o no Gestor de Tarefas) e compile de novo. |
| A janela abre e fecha logo | veja `%TEMP%\sudomake-partition-panic.txt`; corra pela linha de comando com `--sem-admin` para ver o erro. |
| "sem permissão de Administrador" nos discos | aceite o pedido de elevação ao abrir, ou abra o `.exe` com o botão direito → *Executar como administrador*. |
| "Montar como unidade" diz que falta o Dokan | instale o `DokanSetup.exe` (secção 1) e reinicie. |
| Uma unidade ficou "presa" depois de fechar à força | `sudomake-partition desmontar Z` (como Administrador). |
| O GitHub Actions falhou no passo *Release* | confirme que o `build.yml` tem `permissions: contents: write` e que a *tag* começa por `v`. |
| `cargo` não é reconhecido | feche e abra de novo o terminal depois de instalar o Rust, ou adicione `%USERPROFILE%\.cargo\bin` ao PATH. |

## 10. Regras do projecto

* **Tudo é somente leitura**: nenhuma função pode escrever no disco de origem.
* Textos para o utilizador passam sempre por `src/i18n.rs`, nos quatro idiomas.
* Corra `cargo test` antes de publicar; se mexer num leitor (APFS, HFS+, ext4), compare o
  resultado com o 7-Zip ou com a imagem original antes de lançar a versão.
* Os dados de contacto e doação são os de `src/i18n.rs`; mantenha o instalador e o README
  iguais quando os mudar.
