# Engenharia reversa

Notas sobre o comportamento do `Hunter.exe` (versão 1.00, sha256
`663e22cd9938ba6a5f4288c001e77f49558c7a8e76ed0a6d1ccf69472a44a365`), escritas para que
cada formato e cada comportamento portado seja rastreável até o original.

## Método

- A análise roda com o radare2 (`tools/re/analyze.sh <Hunter.exe>`), que gera um índice
  de funções, nomes de tipo e referências a strings em `re-out/`.
- `re-out/` fica **fora do git**: nada do executável, nem desmontagem, nem pseudocódigo
  entra no repositório.
- Os documentos desta pasta descrevem o comportamento **com palavras e endereços**. O
  código do OpenHunted é escrito a partir dessas descrições, não copiado da desmontagem.
- Endereços são endereços virtuais do executável (base `0x400000`).

## Ferramentas

O plano previa o Ghidra, mas o download das releases não estava acessível no ambiente
em que o trabalho começou; usa-se o radare2 5.5 (pacote do Ubuntu) com o módulo Python
`r2pipe`. Quem tiver o Ghidra pode usá-lo nos mesmos endereços.

## Documentos

- [`executavel.md`](executavel.md): mapa geral do executável.
- [`compressao.md`](compressao.md): compressão dos recursos.
- [`anm.md`](anm.md): animações.
- Fases: formato e endereços em `docs/formats/hls.md` e `lev.md`.
- Cenários e interfaces: endereços em [`executavel.md`](executavel.md); formato em `docs/formats/brd.md` e `itf.md`.
