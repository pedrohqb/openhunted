#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Oráculo da auditoria M17: roda o Hunter.exe original no Wine (X virtual), entra numa
fase, aplica uma entrada roteirizada tique a tique e grava o estado dos objetos (lido da
memória do processo) em JSON lines.

Sincronismo: um ponto de parada de hardware (DR0) em 0x4043a3, no laço do jogo
(0x40433c) logo depois da leitura dos dispositivos e antes da física. Em cada parada:
  1. o estado visível é o do fim do tique anterior → vira a linha desse tique;
  2. a entrada do tique seguinte é escrita direto na estrutura do teclado do jogador
     (0x499014[n]: eixos +0x94/+0x96, botões +0x98…+0x9d), que 0x407e2d converte nos bits
     de 0x487a2c dentro da atualização do Jake. Nenhum byte de código é alterado.
Um segundo ponto (DR1) na carga de fase (0x43124b) troca o nome da fase pedida e reinicia
o gerador `rand` (0x460a74) com o estado inicial gravado no executável: execuções
repetidas são idênticas.

Uso:
  oraculo.py setup --cd /caminho/do/cd
  oraculo.py run --level T_J_001 --input "UL,20*R" --ticks 60 --out rastro.jsonl
"""

import argparse
import hashlib
import json
import os
import shutil
import struct
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ptrace_x86 import Mem, Thread  # noqa: E402
import memoria  # noqa: E402

PADRAO_PREFIXO = os.environ.get("ORACULO_PREFIX", "/home/claude/oraculo/prefix")
PADRAO_DISPLAY = os.environ.get("ORACULO_DISPLAY", ":77")
PASTA_JOGO = r"C:\Sierra\Hunter"

BP_TIQUE = 0x4043A3  # laço do jogo: depois de 0x448366 (dispositivos), antes de 0x435e7f
BP_CARGA = 0x43124B  # 0x43124b(nome): carga da fase


def log(*a):
    print("[oráculo]", *a, file=sys.stderr, flush=True)


# --------------------------------------------------------------------------- entrada
# Mesma sintaxe de `ohtool render --input` (crates/ohtool/src/main.rs, parse_input).
BITS = {"L": 0x4, "R": 0x8, "U": 0x1, "D": 0x2, "J": 0x200, "S": 0x100, "X": 0x400,
        "P": 0x40000, "N": 0x800, "M": 0x1000}
SELECT_SHIFT, SELECT_MASK = 24, 0x0700_0000


def tokens(letras):
    out, i = [], 0
    while i < len(letras):
        c = letras[i]
        i += 1
        if c == "W":
            if i < len(letras) and letras[i] in "123456":
                out.append((int(letras[i]) << SELECT_SHIFT) & SELECT_MASK)
                i += 1
            else:
                out.append(0)
        else:
            out.append(BITS.get(c, 0))
    return out


def parse_input(s):
    """Sequência de bits por tique; uma só entrada vale para todos os tiques."""
    if "," in s:
        seq = []
        for tok in s.split(","):
            n, letras = 1, tok
            if "*" in tok:
                a, letras = tok.split("*", 1)
                try:
                    n = int(a.strip())
                except ValueError:
                    n = 1
            b = 0
            for x in tokens(letras):
                b |= x
            seq += [b] * n
        return seq
    return tokens(s)


def entrada_do_tique(seq, t):
    if len(seq) == 1:
        return seq[0]
    return seq[t] if t < len(seq) else 0


def dispositivo(bits):
    """Bits do porte → bytes do teclado do original (+0x8c…+0x9d)."""
    x = 128 * ((1 if bits & 0x8 else 0) - (1 if bits & 0x4 else 0))
    y = 128 * ((1 if bits & 0x2 else 0) - (1 if bits & 0x1 else 0))
    botoes = [bits & 0x100, bits & 0x200, bits & 0x400,  # atirar, pular, furtivo
              bits & 0x1000, bits & 0x800, bits & 0x40000]  # próxima, anterior, soco
    # +0x8c/+0x90: o teclado grava ((eixo + 0x80) >> 8) − 1 (0x4485bf)
    return (struct.pack("<ii", ((x + 0x80) >> 8) - 1, ((y + 0x80) >> 8) - 1)
            + struct.pack("<hh", x, y) + bytes(1 if b else 0 for b in botoes))


# --------------------------------------------------------------------------- Wine
def ambiente(prefixo, display):
    env = dict(os.environ)
    env.update(WINEPREFIX=prefixo, WINEARCH="win32", DISPLAY=display, WINEDEBUG="-all",
               # sem áudio (o jogo segue sem DirectSound) e sem Mono/Gecko
               WINEDLLOVERRIDES="winealsa.drv,winepulse.drv=d;mscoree,mshtml=")
    return env


def pasta_jogo_unix(prefixo):
    return os.path.join(prefixo, "drive_c", "Sierra", "Hunter")


def cmd_setup(a):
    """Cria o prefixo e copia o jogo (os originais do CD não são tocados)."""
    for prog in ("wine", "wineboot", "wineserver", "Xvfb", "xdotool"):
        if not shutil.which(prog):
            sys.exit(f"falta {prog}: rode tools/oraculo/instalar.sh")
    env = ambiente(a.prefix, a.display)
    xvfb = garantir_x(a.display)
    if not os.path.isdir(os.path.join(a.prefix, "drive_c")):
        log("criando o prefixo", a.prefix)
        subprocess.run(["wineboot", "-i"], env=env, check=True)
        subprocess.run(["wineserver", "-w"], env=env)
    dst = pasta_jogo_unix(a.prefix)
    os.makedirs(dst, exist_ok=True)
    src = os.path.join(a.cd, "Hunter")
    for n in os.listdir(src):
        shutil.copy2(os.path.join(src, n), dst)
    # o executável relança a si mesmo se não acha o sierra.inf na pasta corrente (0x4032f3)
    shutil.copy2(os.path.join(a.cd, "sierra.inf"), dst)
    for n in os.listdir(dst):
        os.chmod(os.path.join(dst, n), 0o644)
    # \WINDOWS\SIERRA.INI, seção [Hunter]: InstallPath (precisa ter hunter.exe) e CDPath
    # (onde estão os RESOURCE.*), lidos em 0x403140…0x4031d1
    ini = os.path.join(a.prefix, "drive_c", "windows", "sierra.ini")
    with open(ini, "w", newline="\r\n") as f:
        f.write(f"[Hunter]\nInstallPath={PASTA_JOGO}\nCDPath={PASTA_JOGO}\n")
    if xvfb:
        xvfb.terminate()
    log("pronto:", dst)


def garantir_x(display):
    """Sobe um Xvfb no display pedido se ainda não há um; devolve o processo criado."""
    env = dict(os.environ, DISPLAY=display)
    if subprocess.run(["xdpyinfo"], env=env, capture_output=True).returncode == 0:
        return None
    p = subprocess.Popen(["Xvfb", display, "-screen", "0", "800x600x24", "-nolisten", "tcp"],
                         stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(50):
        time.sleep(0.1)
        if subprocess.run(["xdpyinfo"], env=env, capture_output=True).returncode == 0:
            return p
    sys.exit("Xvfb não subiu")


def escrever_ini(prefixo, dificuldade):
    p = os.path.join(pasta_jogo_unix(prefixo), "Hunter.INI")
    with open(p, "w", newline="\r\n") as f:
        f.write("[Hunter]\nBrightness=1402\nDifficulty=%d\nPlayer1Input=Keyboard1\n"
                "MusicVolume=0\nNoMusicStart=1\n" % dificuldade)


def achar_pid(prefixo, timeout=30):
    """O processo Linux que tem o Hunter.exe mapeado em 0x400000."""
    alvo = os.path.realpath(os.path.join(pasta_jogo_unix(prefixo), "Hunter.exe")).lower()
    t0 = time.time()
    while time.time() - t0 < timeout:
        for d in os.listdir("/proc"):
            if not d.isdigit():
                continue
            try:
                with open(f"/proc/{d}/maps") as f:
                    for ln in f:
                        if ln.startswith("00400000-") and ln.rstrip().lower().endswith(alvo):
                            return int(d)
            except OSError:
                continue
        time.sleep(0.2)
    sys.exit("o Hunter.exe não apareceu")


class Xdo:
    def __init__(self, display):
        self.env = dict(os.environ, DISPLAY=display)

    def __call__(self, *args):
        return subprocess.run(["xdotool", *args], env=self.env, capture_output=True,
                              text=True).stdout

    def mouse(self):
        out = self("getmouselocation", "--shell")
        d = dict(ln.split("=", 1) for ln in out.split() if "=" in ln)
        return int(d.get("X", -1)), int(d.get("Y", -1))

    def janela(self):
        return self("search", "--name", "^Hunter Hunted$").split()

    def foto(self, caminho):
        subprocess.run(["import", "-window", "root", caminho], env=self.env,
                       capture_output=True)


# --------------------------------------------------------------------------- execução
def esperar(th, segundos):
    """Deixa o jogo rodar `segundos` atendendo aos sinais da thread rastreada; devolve o
    EIP se um ponto de parada foi alcançado (a thread fica parada), senão None."""
    try:
        return th.wait_break(timeout=segundos)
    except TimeoutError:
        return None


def cmd_run(a):
    seq = parse_input(a.input)
    if any(b & SELECT_MASK for b in seq):
        log("aviso: W1…W6 é extensão do porte; o original não tem escolha direta (ignorado)")
    env = ambiente(a.prefix, a.display)
    xvfb = garantir_x(a.display)
    subprocess.run(["wineserver", "-k"], env=env, capture_output=True)
    time.sleep(0.5)
    escrever_ini(a.prefix, a.difficulty)
    jogo = subprocess.Popen(["wine", "Hunter.exe"], cwd=pasta_jogo_unix(a.prefix), env=env,
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                            start_new_session=True)
    xdo = Xdo(a.display)
    th = None
    saida = open(a.out, "w") if a.out != "-" else sys.stdout
    try:
        pid = achar_pid(a.prefix)
        mem = Mem(pid)
        log("pid", pid)
        th = Thread(pid)  # a thread inicial roda o WinMain e o laço do jogo
        th.interrupt()
        th.set_breakpoints([BP_TIQUE, BP_CARGA])
        th.cont()
        rng_ini = memoria.rng_inicial(os.path.join(pasta_jogo_unix(a.prefix), "Hunter.exe"))

        # --- abertura e menu: Esc interrompe a abertura (e é ignorado no menu); o menu
        # e o briefing põem o ponteiro no centro do botão padrão (New Game / GO).
        parado = (3, 3)
        xdo("mousemove", str(parado[0]), str(parado[1]))
        fase = None
        t0 = time.time()
        estado = "abertura"
        while True:
            if time.time() - t0 > a.timeout:
                raise RuntimeError(f"tempo esgotado na etapa {estado}")
            eip = esperar(th, 0.3)
            if eip == BP_CARGA:
                fase = trocar_fase(mem, th, a.level)
                memoria.escrever_rng(mem, rng_ini)
                th.cont()
                estado = "briefing"
                pos_menu = xdo.mouse()
                continue
            if eip == BP_TIQUE:
                break  # GO: o laço do jogo começou (a thread está parada no tique 0)
            if estado == "abertura":
                if xdo.mouse() != parado:
                    estado = "menu"
                    log("menu principal; New Game")
                    time.sleep(0.3)
                    xdo("click", "1")
                    continue
                if xdo.janela():
                    xdo("key", "Escape")
            elif estado == "briefing":
                if xdo.mouse() != pos_menu:
                    log("briefing; GO")
                    time.sleep(0.3)
                    xdo("click", "1")
                    estado = "go"
        log("fase", fase, "começou;", f"{time.time() - t0:.1f} s de menus")

        # --- tiques
        meta = {"meta": {
            "fonte": "original", "fase": fase, "input": a.input, "ticks": a.ticks,
            "dificuldade": a.difficulty, "exe_md5": md5(os.path.join(
                pasta_jogo_unix(a.prefix), "Hunter.exe")),
            "rng_tique0": memoria.rng_indices(mem),
            "campos": memoria.CAMPOS}}
        print(json.dumps(meta, ensure_ascii=False), file=saida)
        ids = {}
        jogador = memoria.achar_jogador(mem)
        if jogador is None:
            raise RuntimeError("sem jogador na lista de objetos")
        dev = mem.u32(0x499014 + 4 * mem.i32(jogador + 0x198))
        if a.inicial:
            print(json.dumps(memoria.quadro(mem, -1, None, ids, a.todos)), file=saida)
        # o estado do `rand` no tique 0 também é fixado: o briefing sorteia conforme o
        # tempo que fica aberto
        memoria.escrever_rng(mem, rng_ini)
        t = 0
        pausados = 0
        modal = 0
        bits = entrada_do_tique(seq, 0)
        mem.write(dev + 0x8C, dispositivo(bits))
        th.cont()
        while t < a.ticks:
            eip = esperar(th, 0.8)
            if eip is None:
                # sem tique em 0,8 s: diálogo modal dentro do tique (dica do "mago",
                # docs/re/dicas.md); fecha soltando uma tecla que o jogo não usa
                modal += 1
                if modal > 40:
                    raise RuntimeError(f"o jogo parou no tique {t}")
                xdo("key", "--delay", "100", "Escape")
                continue
            if eip != BP_TIQUE:
                th.cont()
                continue
            if mem.i32(0x47119C):  # pausado: o tique não simulou
                pausados += 1
                mem.write(dev + 0x8C, dispositivo(bits))
                th.cont()
                continue
            q = memoria.quadro(mem, t, bits, ids, a.todos)
            if modal:
                q["modal"] = True  # este tique abriu uma dica (o jogo esperou a tecla)
                modal = 0
            print(json.dumps(q, separators=(",", ":")), file=saida)
            if a.shots and t % a.shot_every == 0:
                xdo.foto(os.path.join(a.shots, f"t{t:05d}.png"))
            t += 1
            bits = entrada_do_tique(seq, t)
            mem.write(dev + 0x8C, dispositivo(bits))
            th.cont()
        log(f"{t} tiques gravados em {a.out}" + (f" ({pausados} pausados)" if pausados else ""))
    finally:
        if saida is not sys.stdout:
            saida.close()
        if th is not None:
            th.detach()
        if not a.keep:
            subprocess.run(["wineserver", "-k"], env=env, capture_output=True)
            try:
                os.killpg(jogo.pid, 9)
            except OSError:
                pass
            if xvfb:
                xvfb.terminate()


def trocar_fase(mem, th, nivel):
    """No início de 0x43124b(nome): [esp+4] aponta o nome "HUNTER.HLS|T_J_001.lev"."""
    esp = th.regs().rsp & 0xFFFFFFFF
    p = mem.u32(esp + 4)
    nome = mem.read(p, 64).split(b"\0")[0].decode("latin-1")
    if nivel:
        base = nivel.upper()
        if not base.endswith(".LEV"):
            base += ".lev"
        novo = (nome.split("|")[0] + "|" + base) if "|" in nome else base
        mem.write(p, novo.encode("latin-1") + b"\0")
        log(f"fase {nome} → {novo}")
        nome = novo
    return nome.split("|")[-1].rsplit(".", 1)[0].upper()


def md5(p):
    with open(p, "rb") as f:
        return hashlib.md5(f.read()).hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--prefix", default=PADRAO_PREFIXO, help="WINEPREFIX (fora do repositório)")
    ap.add_argument("--display", default=PADRAO_DISPLAY, help="display do Xvfb")
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("setup", help="cria o prefixo do Wine e copia o jogo do CD")
    s.add_argument("--cd", required=True, help="raiz do CD (com Hunter/ e sierra.inf)")
    r = sub.add_parser("run", help="roda uma fase com entrada roteirizada")
    r.add_argument("--level", default="T_J_001", help="fase (ex.: T_J_001)")
    r.add_argument("--input", default="", help='entrada, sintaxe do ohtool ("UL,20*R,J")')
    r.add_argument("--ticks", type=int, default=60)
    r.add_argument("--out", default="-", help="JSON lines (- = saída padrão)")
    r.add_argument("--difficulty", type=int, default=1, help="0 fácil, 1 médio, 2 difícil")
    r.add_argument("--todos", action="store_true",
                   help="inclui objetos de lógica sem animação (tipos 0, 27, 30, 73)")
    r.add_argument("--inicial", action="store_true", help="grava também o estado antes do tique 0")
    r.add_argument("--shots", help="pasta para capturas de tela")
    r.add_argument("--shot-every", type=int, default=10)
    r.add_argument("--timeout", type=float, default=120, help="limite dos menus (s)")
    r.add_argument("--keep", action="store_true", help="deixa o jogo aberto no fim")
    a = ap.parse_args()
    if a.cmd == "setup":
        cmd_setup(a)
    else:
        if a.shots:
            os.makedirs(a.shots, exist_ok=True)
        cmd_run(a)


if __name__ == "__main__":
    main()
