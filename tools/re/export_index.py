#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Exporta um índice do Hunter.exe com o radare2 (via r2pipe).

Uso: export_index.py <Hunter.exe> <pasta de saída>

Gera, na pasta de saída (que deve ficar fora do git, em re-out/):
  functions.tsv     endereço, nome, tamanho
  types.tsv         nomes de tipo do RTTI do Borland ("Classe *") e seus endereços
  string-xrefs.tsv  endereço da string, texto, função que a referencia
"""
import os
import sys

import r2pipe


def main() -> None:
    exe, out = sys.argv[1], sys.argv[2]
    os.makedirs(out, exist_ok=True)
    r2 = r2pipe.open(exe, flags=["-2", "-e", "bin.cache=true"])
    r2.cmd("aaa")
    funcs = r2.cmdj("aflj") or []
    with open(os.path.join(out, "functions.tsv"), "w") as f:
        f.write("addr\tname\tsize\n")
        for fn in sorted(funcs, key=lambda x: x["offset"]):
            f.write(f"{fn['offset']:#010x}\t{fn['name']}\t{fn['size']}\n")
    strings = [s for s in (r2.cmdj("izzj") or []) if s.get("section") in ("CODE", "DATA")]
    with open(os.path.join(out, "types.tsv"), "w") as t, open(
        os.path.join(out, "string-xrefs.tsv"), "w"
    ) as x:
        t.write("addr\ttype\n")
        x.write("str_addr\ttext\tfunc_addr\tfunc_name\n")
        for s in strings:
            text = s["string"].replace("\t", " ").replace("\n", "\\n")
            if text.endswith(" *"):
                t.write(f"{s['vaddr']:#010x}\t{text}\n")
            for ref in r2.cmdj(f"axtj @ {s['vaddr']}") or []:
                x.write(
                    f"{s['vaddr']:#010x}\t{text}\t{ref.get('fcn_addr', 0):#010x}\t"
                    f"{ref.get('fcn_name', '')}\n"
                )
    r2.quit()


if __name__ == "__main__":
    main()
