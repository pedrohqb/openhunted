# Mapa do `Hunter.exe`

## Visão geral

- PE32 i386, compilado com **Borland C++ 1995** (seções `CODE`, `DATA`, `.INIT`,
  `.idata`, `.edata`, `.reloc`, `.rsrc`; base `0x400000`).
- Usa uma biblioteca gráfica própria da Dynamix com classes `GFX*`: `GFXDevManager`,
  `GFXDevWinG`, `GFXBuffer`, `GFXBitmap`, `GFXBitmapArray`, `GFXPalette`, `GFXFont`.
  As mensagens de erro dessas classes continuam no executável e ajudam a localizar funções.
- Fluxos de leitura: `StreamIO`, `FileRStream`, `MemRWStream`, `LZHRStream`, `LZHuffman`,
  `RLERStream` (nomes de tipo do RTTI do Borland, na forma `"Classe *"`).
- A interface usa funções `tb_*` (exportadas: `tb_interface_process_edit_message`,
  `tb_window_process_parent`, `tb_window_process_child`, `tb_window_process_dialog`) e
  `process_deathmatch_interface`. O prefixo "TB" aparece também nos dados (`ANM:` começa por
  `"TB"`, `ITF:` por `"TBO"`).
- Importa só GDI32, USER32, KERNEL32, WINMM (timer `timeSetEvent`, joystick, `mciSendString`
  para as faixas de áudio do CD), DSOUND, COMDLG32, ADVAPI32. Sem DirectDraw, sem rede.

## Arquivos e recursos

| Endereço | Papel |
|---|---|
| `0x004030ba` | Inicialização: monta os caminhos `%s\resource.map`, `%s\resource.001`, `%s\resource.002` e `%s\hunter.hls` |
| `0x0045e105` | Abre um recurso pelo nome e devolve um fluxo posicionado nos dados |
| `0x0045e18e` | Procura um bloco `TAG:` no fluxo do recurso (usado com `"BMP:"`) |
| `0x004492f4` | Leitura de fluxo: `read(buf, tam, n, fluxo)` |
| `0x004493ef` | Posicionamento relativo no fluxo (`seek(fluxo, delta, 1)`) |
| `0x0045deac` | Lê `u8` do fluxo |
| `0x0045dee6` | Lê `u16` do fluxo |
| `0x0045df21` | Lê `u32` do fluxo |
| `0x0045deba` | Lê o byte de tipo de um quadro de bitmap |

## Formatos

| Endereço | Papel |
|---|---|
| `0x0045b914` | Lê um recurso `BMP:` inteiro: versão (≤ 1000), número de quadros, tabela de offsets, e para cada quadro largura, altura e dados crus ou comprimidos |
| `0x0045bbec` | Outra leitura de `BMP:` (mesma estrutura de quadro; chama o descompressor) |
| `0x0045af70` | Leitor de bitmap no formato DGDS antigo (`BMP:INF:`, `BMP:BIN:`, `BMP:VGA:`), sem uso visto nos dados |
| `0x00453f5f` | Cria um `GFXBitmap` (largura, altura, 8 bits) |
| `0x0044f4ed` | Passo de linha (*pitch*) de um `GFXBitmap` |
| `0x00466d1c` | Descompressor genérico: lê tipo, tamanhos e despacha para o método (ver `compressao.md`) |
| `0x00467050` | Método 3 de compressão (LZ com fluxo de flags separado) |
| `0x004673e8` | Método 0 (cópia) |
| `0x00459cce` | Construtor do fluxo usado no método 1 |
| `0x00458f22` | Construtor de `LZHRStream` (método 2) |
| `0x004593f4` | Construtor de `LZHuffman` |
| `0x00459a50` | Fluxo em memória sobre um buffer (entrada dos métodos 1 e 2) |
| `0x00455f55` | `GFXPalette::read` (formato próprio) e `0x00456087` `GFXPalette::readMSPal` (RIFF PAL) |
| `0x00455719` | `GFXFont::read` (fontes `.PFT`); `0x004555d5` `GFXFont::load` |
| `0x00453712` | `GFXBitmapArray::read` |
| `0x00447a16` | Leitor de animação: blocos `ANM:` e `TAG:` |
| `0x00446ccc` | Leitura de bloco `TAG:` (animações) |
| `0x00461310` | Leitor de interface `ITF:` |
| `0x00430afb` | Leitor de cenário (*border*): blocos `HED:` e `BDR:` |
| `0x0043124b` | Leitor de fase: bloco `LEV:` (referencia `I_J_001.LEV`) |
| `0x004406d3` | Seleção de fases: lista `*.hls` / `*.lev`, abre `HUNTER.HLS` |
| `0x0040bff8` | Carrega os cenários dos objetos (`obb.brd`, `obk00.brd`, `ogr2.brd`…) |
| `0x0043d514` | Carrega `DAMAGE.TBL` |
| `0x0043be18` | Carrega `DLS.TBL` |
| `0x0043c0ce` | Carrega `SOUND.TBL` |
| `0x0040107c` | Carrega a fonte `garam12.pft` |
| `0x0043a18f` | Carrega a fonte `credits.pft` |

## Questões abertas

- Algoritmo do hash do nome no `RESOURCE.MAP` (dentro de `0x0045e18e` e das funções que ela
  chama, `0x0045e060`/`0x0045e0cb`/`0x0045e162`).
- Em que dados os métodos 1 e 2 de compressão aparecem (nenhum bitmap os usa).
