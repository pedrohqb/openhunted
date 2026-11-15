# SPDX-License-Identifier: GPL-3.0-or-later
"""Leitura do estado do Hunter.exe 1.00 (base 0x400000) pela memória do processo.

Endereços e deslocamentos documentados em docs/re/ (personagem.md, logica-fase.md,
armas.md, dano-vida.md, minas.md).
"""

import struct

LISTA = 0x488E7C       # cabeça da lista de objetos (encadeada por +0x1c, ordenada por +0x1b4)
TIQUE = 0x4879C4       # contador de tiques do laço (0x404541)
ENTRADA = 0x487A2C     # bits de entrada da última atualização (0x407e2d)
RNG_I, RNG_J, RNG_T = 0x483918, 0x48391C, 0x483920  # rand 0x460a74: índices e 56 palavras
TAM_OBJ = 0x2B0
LOGICA = {0, 27, 30, 73}  # objetos de lógica sem animação

# nome curto → (deslocamento, formato); a ordem é a das colunas do rastro
CAMPOS = {
    "tipo": (0x18, "i"), "x": (0x14C, "i"), "y": (0x150, "i"),
    "vx": (0x164, "i"), "vy": (0x168, "i"),        # 1/8 px por tique
    "fl": (0x178, "I"), "cam": (0x17C, "i"), "est": (0x180, "i"),
    "q": (0x184, "i"), "rot": (0x1EC, "i"), "esp": (0x194, "i"),
    "vida": (0x258, "i"), "arm": (0x25C, "i"),
}
# só para jogadores (+0x19c ≠ 0)
CAMPOS_JOGADOR = {
    "arma": (0x254, "i"), "veneno": (0x270, "i"), "lado": (0x288, "i"),
    "recarga": (0x294, "i"), "c260": (0x260, "i"), "c264": (0x264, "i"),
    "c268": (0x268, "i"),
}


def _campo(blob, ofs, fmt):
    return struct.unpack_from("<" + fmt, blob, ofs)[0]


MORTOS = 0x488E80      # lista dos destruídos (0x43cfcf a percorre junto)
CELULAS = 0x488E84     # vetor de ponteiros de célula (gx·gy·gz); a lista da célula em +0xd0
GRADE = (0x488E6C, 0x488E70, 0x488E74)


def _cadeia(mem, o, limite=5000):
    n = 0
    while o and n < limite:
        yield o
        o = mem.u32(o + 0x1C)
        n += 1


def objetos(mem, mortos=False):
    """(endereço, lista) de todos os objetos, como 0x43cfcf os percorre: a lista global
    ("g", ordenada por +0x1b4), as listas das células ("c": itens, cenário animado…) e, se
    pedido, a dos destruídos ("m")."""
    for o in _cadeia(mem, mem.u32(LISTA)):
        yield o, "g"
    gx, gy, gz = (mem.i32(a) for a in GRADE)
    n = gx * gy * gz
    if 0 < n < 1_000_000:
        vet = struct.unpack(f"<{n}I", mem.read(mem.u32(CELULAS), 4 * n))
        for c in vet:
            if c:
                for o in _cadeia(mem, mem.u32(c + 0xD0)):
                    yield o, "c"
    if mortos:
        for o in _cadeia(mem, mem.u32(MORTOS)):
            yield o, "m"


def achar_jogador(mem):
    for o, _ in objetos(mem):
        if mem.i32(o + 0x18) in (1, 2) and mem.i32(o + 0x19C):
            return o
    return None


def ler(mem, o):
    b = mem.read(o, TAM_OBJ)
    d = {k: _campo(b, ofs, f) for k, (ofs, f) in CAMPOS.items()}
    if _campo(b, 0x19C, "i") and d["tipo"] in (1, 2):
        d["jogador"] = _campo(b, 0x19C, "i")
        d.update({k: _campo(b, ofs, f) for k, (ofs, f) in CAMPOS_JOGADOR.items()})
        pares = struct.unpack_from("<60i", b, 0x28)
        d["inv"] = {i: [pares[2 * i], pares[2 * i + 1]] for i in range(30)
                    if pares[2 * i] or pares[2 * i + 1]}
    return d


VISTAS = 0x493B70


def quadro(mem, t, bits, ids, todos):
    """Uma linha do rastro. `ids` dá a cada endereço um número estável (ordem de chegada)."""
    objs = []
    for o, lista in objetos(mem, mortos=True):
        d = ler(mem, o)
        d["lista"] = lista
        if not todos and d["tipo"] in LOGICA:
            continue
        if o not in ids:
            ids[o] = len(ids)
        d["id"] = ids[o]
        objs.append(d)
    return {"t": t, "in": bits, "in_jogo": mem.i32(ENTRADA), "tique": mem.i32(TIQUE),
            "rng": rng_indices(mem), "vista": vista(mem), "obj": objs}


def vista(mem):
    """Câmera da 1ª vista (`[0x493b70]`, `0x436386`): centro x, y no mundo e distância `d`."""
    v = mem.u32(VISTAS)
    return [mem.i32(v), mem.i32(v + 4), mem.i32(v + 8)] if v else None


def rng_indices(mem):
    return [mem.i32(RNG_I), mem.i32(RNG_J)]


def rng_inicial(exe):
    """Estado do `rand` como está no arquivo (seção DATA, VA 0x471000 ↔ arquivo 0x70000)."""
    with open(exe, "rb") as f:
        f.seek(0x70000 + RNG_I - 0x471000)
        return f.read(8 + 56 * 4)


def escrever_rng(mem, estado):
    mem.write(RNG_I, estado)
