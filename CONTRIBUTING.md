# Contribuir com o SUDOMAKE Partition

Obrigado pelo interesse! Toda ajuda é bem-vinda: relatos de erro, testes com discos reais,
documentação, tradução e código. O projecto é feito em Angola e aberto ao mundo.

*English: contributions are welcome (issues, real-disk test reports, docs, translations, code).
Build with `cargo build --release`; run `cargo test` before a pull request; keep everything
read-only. Code identifiers are in English, user-facing text goes through `src/i18n.rs`.*

## Relatar um problema

Abra uma *issue* em <https://github.com/arturjose0/sudomake-partition/issues> com:

* o que tentou (comando ou botão) e o que aconteceu;
* a saída de `sudomake-partition info <origem>` (ou uma captura do ecrã inicial);
* se um ficheiro não abre, a saída de `sudomake-partition verificar <origem> <pasta>`;
* versão do Windows e, para montagem, a versão do Dokan (`C:\Program Files\Dokan`).

Não envie imagens de disco com dados pessoais. Para reproduzir um problema de formato, uma
imagem pequena criada de propósito (por exemplo com `mke2fs -d` ou `hdiutil`) é ideal.

## Compilar

```text
rustup toolchain install stable-x86_64-pc-windows-gnu   # ou use a toolchain MSVC padrão
cargo +stable-x86_64-pc-windows-gnu build --release
cargo +stable-x86_64-pc-windows-gnu test
```

`build.cmd` faz tudo isso e copia os executáveis para a raiz do projecto. O instalador é gerado
com `ISCC.exe installer\sudomake-partition.iss` (Inno Setup 6). Passo a passo completo em
[GUIA-DO-DESENVOLVEDOR.md](GUIA-DO-DESENVOLVEDOR.md).

## Organização do código

| Ficheiro | Conteúdo |
|---|---|
| `src/device.rs` | acesso a discos físicos (`\\.\PhysicalDriveN`), imagens, cache e dispositivos mapeados |
| `src/partition.rs` | GPT, MBR, Apple Partition Map e detecção do sistema de ficheiros |
| `src/apfs.rs` | leitor APFS (checkpoints, object map, árvores B, extents, xattrs) |
| `src/hfsplus.rs` | leitor HFS+/HFSX (catálogo, extents, atributos, hard links) |
| `src/ext4.rs` | leitor ext2/3/4 (extents, blocos indirectos, inline data) |
| `src/lvm.rs` | metadados LVM2 e mapeamento de volumes lógicos lineares |
| `src/dmg.rs` | imagens DMG (UDIF) com zlib, bzip2, ADC, LZFSE, LZMA |
| `src/decmpfs.rs`, `src/lzvn.rs` | ficheiros comprimidos do macOS |
| `src/fs.rs` | interface comum (`FileSystem`, `Entry`, `FileReader`, capacidade) |
| `src/osdetect.rs` | sistema instalado em cada volume e finalidade das outras partições |
| `src/open.rs` | abertura de origens e escolha de partição/volume |
| `src/copy.rs` | cópia recursiva com registo, retomada, progresso e cancelamento |
| `src/fsservice.rs`, `src/dokan.rs` | montagem como unidade do Windows via Dokan |
| `src/i18n.rs` | todos os textos da interface em PT-AO, EN, FR, ES e os dados de contacto/doação |
| `src/main.rs` | linha de comando (`sudomake-partition.exe`) |
| `src/bin/gui.rs` | interface gráfica (`SudomakePartition.exe`, native-windows-gui, azulejos pintados à mão) |
| `installer/` | script do Inno Setup, ícone e textos "sobre" por idioma |

## Traduzir

Todos os textos estão em `src/i18n.rs`, numa tabela `chave => [pt, en, fr, es]`. Para acrescentar
um idioma: adicione a variante em `Lang`, uma coluna na macro `table!` e o código em
`from_langid`. O instalador tem as suas mensagens em `installer/sudomake-partition.iss`
(`[Messages]` e `[CustomMessages]`) e um `installer/sobre-<idioma>.txt`.

## Ideias de melhorias

* Leitores para **XFS**, **Btrfs** e **F2FS**.
* Abrir volumes **LUKS** e **FileVault** com a senha do utilizador.
* Replay do journal do ext4 e leitura de **snapshots** APFS (backups do Time Machine).
* Segmentos LVM em faixas (*striped*) e em vários discos.
* Copiar também permissões, atributos estendidos e resource forks.
* Mais idiomas.

## Regras simples

* Tudo é **somente leitura**: nunca escreva no disco de origem.
* Mantenha o código sem `unsafe` fora de `device.rs`, `dokan.rs` e da interface.
* Corra `cargo test` antes de enviar o *pull request* e descreva como testou (imagem usada,
  comparação com 7-Zip, etc.).
* Textos para o utilizador passam por `i18n.rs` (nos quatro idiomas); nomes de código em inglês.
