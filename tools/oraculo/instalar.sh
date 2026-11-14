#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Instala o que o oráculo precisa (Ubuntu 24.04, como root) e prepara o prefixo do Wine.
# Uso: tools/oraculo/instalar.sh /caminho/do/cd [prefixo]
# O CD é a cópia do próprio usuário (pasta com Hunter/ e sierra.inf); nada dele vai para o
# repositório nem é alterado: os arquivos são copiados para dentro do prefixo.
set -euo pipefail
cd_dir=${1:?uso: instalar.sh /caminho/do/cd [prefixo]}
prefixo=${2:-/home/claude/oraculo/prefix}

# Wine de 32 bits (o Hunter.exe é PE32), xdotool para os menus, ImageMagick para as
# capturas, x11-utils para o xdpyinfo; o Xvfb já vem na imagem (senão: apt install xvfb)
if ! command -v wine >/dev/null || ! dpkg -s wine32:i386 >/dev/null 2>&1; then
    dpkg --add-architecture i386
    apt-get update -q
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
        wine wine32:i386 xdotool x11-utils imagemagick xauth
fi
command -v Xvfb >/dev/null || DEBIAN_FRONTEND=noninteractive apt-get install -y xvfb

python3 "$(dirname "$0")/oraculo.py" --prefix "$prefixo" setup --cd "$cd_dir"
