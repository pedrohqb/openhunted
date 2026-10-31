#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-or-later
# Análise reprodutível do Hunter.exe com o radare2.
# Uso: tools/re/analyze.sh /caminho/Hunter.exe
# Requer: radare2 (testado com 5.5) e o módulo Python r2pipe.
# A saída vai para re-out/, que fica fora do git.
set -eu
exe="$1"
here=$(dirname "$0")
out="$here/../../re-out"
mkdir -p "$out"
sha=$(sha256sum "$exe" | cut -d' ' -f1)
if [ "$sha" != "663e22cd9938ba6a5f4288c001e77f49558c7a8e76ed0a6d1ccf69472a44a365" ]; then
    echo "aviso: Hunter.exe diferente da versão 1.00 conhecida ($sha)" >&2
fi
python3 -I "$here/export_index.py" "$exe" "$out"
wc -l "$out"/functions.tsv "$out"/types.tsv "$out"/string-xrefs.tsv
