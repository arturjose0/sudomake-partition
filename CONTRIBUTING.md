# Contribuir com o macread

Obrigado pelo interesse! Toda ajuda é bem-vinda: relatos de erro, testes com discos reais,
documentação, tradução e código.

## Relatar um problema

Abra uma *issue* com:

* o que você tentou (comando ou botão) e o que aconteceu;
* a saída de `macread info <origem>` (ou uma captura da árvore da interface);
* se um arquivo não abre, a saída de `macread verificar <origem> <pasta>`;
* versão do Windows e, para montagem, a versão do Dokan (`C:\Program Files\Dokan`).

Não envie imagens de disco com dados pessoais. Para reproduzir um problema de formato,
uma imagem pequena criada de propósito (por exemplo com `mke2fs -d` ou `hdiutil`) é ideal.

## Compilar

```text
rustup toolchain install stable-x86_64-pc-windows-gnu   # ou use a toolchain MSVC padrão
cargo +stable-x86_64-pc-windows-gnu build --release
cargo +stable-x86_64-pc-windows-gnu test
```

`build.cmd` faz tudo isso e copia os executáveis para a raiz do projeto.

## Organização do código

| Arquivo | Conteúdo |
|---|---|
| `src/device.rs` | acesso a discos físicos (`\\.\PhysicalDriveN`), imagens, cache e dispositivos mapeados |
| `src/partition.rs` | GPT, MBR, Apple Partition Map e detecção do sistema de arquivos |
| `src/apfs.rs` | leitor APFS (checkpoints, object map, árvores B, extents, xattrs) |
| `src/hfsplus.rs` | leitor HFS+/HFSX (catálogo, extents, atributos, hard links) |
| `src/ext4.rs` | leitor ext2/3/4 (extents, blocos indiretos, inline data) |
| `src/lvm.rs` | metadados LVM2 e mapeamento de volumes lógicos lineares |
| `src/dmg.rs` | imagens DMG (UDIF) com zlib, bzip2, ADC, LZFSE, LZMA |
| `src/decmpfs.rs`, `src/lzvn.rs` | arquivos comprimidos do macOS |
| `src/fs.rs` | interface comum (`FileSystem`, `Entry`, `FileReader`) |
| `src/open.rs` | abertura de origens e escolha de partição/volume |
| `src/copy.rs` | cópia recursiva com log, retomada, progresso e cancelamento |
| `src/fsservice.rs`, `src/dokan.rs` | montagem como unidade do Windows via Dokan |
| `src/main.rs` | linha de comando |
| `src/bin/macread-gui.rs` | interface gráfica (native-windows-gui) |

## Ideias de melhorias

* Leitores para **XFS**, **Btrfs** e **F2FS**.
* Abrir volumes **LUKS** e **FileVault** com a senha do usuário.
* Replay do journal do ext4 e leitura de **snapshots** APFS (backups do Time Machine).
* Segmentos LVM em faixas (*striped*) e em vários discos.
* Copiar também permissões, atributos estendidos e resource forks.
* Tradução da interface para outros idiomas.

## Regras simples

* Tudo é **somente leitura**: nunca escreva no disco de origem.
* Mantenha o código sem `unsafe` fora de `device.rs`, `dokan.rs` e da interface.
* Rode `cargo test` antes de enviar o *pull request* e descreva como testou
  (imagem usada, comparação com 7-Zip, etc.).
* Mensagens ao usuário em português; nomes de código em inglês.
