# Formatos de dados do Hunter Hunted 1.00

Estado da leitura pelo `openhunted-formats`. "Lido" quer dizer que todos os arquivos daquele
tipo na versão 1.00 passam pelo leitor nos testes com os dados reais.

| Formato | Onde | Estado | Documento |
|---|---|---|---|
| Volume de recursos | `RESOURCE.MAP`, `.001`, `.002` | lido (2.312 entradas) | [`volume.md`](volume.md) |
| Blocos `TAG:` | recursos próprios | lido | [`blocos.md`](blocos.md) |
| Compressão | quadros de bitmap | lido (métodos 0 e 3; 1 e 2 sem uso) | [`compressao.md`](compressao.md) |
| Bitmaps `.BMP` | arquivo de recursos | lido (1.654) | [`bmp.md`](bmp.md) |
| Paletas `.PAL` | arquivo de recursos | lido (26) | [`pal-wav-res.md`](pal-wav-res.md) |
| Sons `.WAV` | arquivo de recursos | lido (280) | [`pal-wav-res.md`](pal-wav-res.md) |
| Textos `.RES` | arquivo de recursos | lido (13) | [`pal-wav-res.md`](pal-wav-res.md) |
| Animações `.ANM` | arquivo de recursos | lido (87); significado de parte dos opcodes pendente | [`anm.md`](anm.md) |
| Cenários `.BRD` | arquivo de recursos | lido (225); significado dos tipos pendente | [`brd.md`](brd.md) |
| Interfaces `.ITF` | arquivo de recursos | lido (16) | [`itf.md`](itf.md) |
| Fontes `.PFT` | arquivo de recursos | lido (5) | [`pft.md`](pft.md) |
| Arquivo de fases | `hunter.hls` | lido (201 entradas) | [`hls.md`](hls.md) |
| Lista de missões | `ENGLEVEL.TXT` | lido (100 missões) | [`lev.md`](lev.md) |
| Fases `.LEV` | `hunter.hls` | lido (100): cabeçalho, objetos, células, listas PR e DC; significado de parte dos campos pendente | [`lev.md`](lev.md) |
| Tabelas `.TBL` | `DAMAGE.TBL`, `DLS.TBL`, `SOUND.TBL` | decodificado (sem leitor) | [`tbl.md`](tbl.md) |
| Esqueletos `.SKE` | `JAKE.SKE`, `MINOTAUR.SKE` | decodificado | [`ske.md`](ske.md) |
| `PLAY.VCR` | arquivo de recursos | **pendente** | — |
| Vídeos | `Movies/*.AVI` (Indeo 3.2 + PCM 8 bits) | identificado (FFmpeg lê) | [`../re/videos-musica.md`](../re/videos-musica.md) |
| Música | faixas de áudio do CD; `SX_3000.WAV` nos menus | identificado | [`../re/videos-musica.md`](../re/videos-musica.md) |
| Jogo salvo | `*.hhg` (fora do CD) | decodificado | [`salvamento.md`](salvamento.md) |
| Lista de heróis | `hunter.fam` (fora do CD) | decodificado | [`salvamento.md`](salvamento.md) |

Engenharia reversa e endereços no executável: [`docs/re/`](../re/).
