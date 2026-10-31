# Animações (engenharia reversa)

Leitor: `0x00447a16`. Abre o recurso (`0x0045e105`), procura o bloco `ANM:` (`0x0045e18e`)
e confere a assinatura `"TB"` byte a byte. Lê cinco `u16` — versão, `A`, `B`, `C` e `D` — e
recusa versão acima de 1004. Aloca uma estrutura de `24 + 2A + 12B + 4(C+1) + D` bytes, com
ponteiros para as quatro tabelas.

Depois lê, nesta ordem:

1. dois `u16` (guardados nos campos `+0x10` e `+0x14` da estrutura): largura e altura de
   referência da animação;
2. se versão ≥ 1002, três `s16` que o leitor **descarta**;
3. `A` × `u16`;
4. `B` registros: quatro `s16`, um `u16` (offset em palavras no roteiro, convertido em
   ponteiro) e, se versão ≥ 1002, mais um `s16` descartado;
5. `C` pares de `u16`;
6. o roteiro: `D / 2` palavras organizadas em instruções. Cada instrução começa por um `s16`
   opcode e tem o tamanho total (em palavras, opcode incluído) dado pela tabela em
   `0x00480608`: `[1, 1, 6, 7, 6, 2, 2, 6, 4, 1, 1]` para os opcodes 0 a 10. A tabela não tem
   checagem de limite: um opcode fora de 0–10 leria lixo.
7. Em versões anteriores a 1002, o último argumento do opcode 2, quando não é 0 nem 3, passa
   por `xor 3` (conversão de formato antigo). Todos os ANM da versão 1.00 são versão 1004.

Em seguida lê o bloco `TAG:` (`0x00460410`) com os nomes dos bitmaps.

## Significado (provisório, a confirmar na M2)

- Registros `B` = quadros: retângulo (x, y, largura, altura) relativo à origem da animação e o
  início do roteiro do quadro.
- Pares `C` = rótulos: (quadro inicial, id do rótulo); o nome do rótulo vem do bloco `LAB:`.
- Opcode 0: fim do quadro. Opcode 1: fim da animação. Opcode 2 (5 argumentos): desenha o
  quadro `arg1` do bitmap de id `arg0` (ids do bloco `TAG:`), em `arg2, arg3`, com
  `arg4` de opções. Opcode 8 (3 argumentos): o primeiro parece ser o número de um som
  (`3099`, `3095` em `SABOUT.ANM`). Opcodes 3 e 4: a decifrar.
