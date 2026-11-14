#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Compara um rastro do original (`oraculo.py run --out`) com o do porte
(`ohtool render --trace-json`), tique a tique.

Jogador: posição, velocidade (1/8 px), rótulo e quadro, camada, vida, armadura, arma,
veneno, lado e inventário. O estado numérico do original (`+0x180`) e o nome do estado do
porte aparecem lado a lado (não se comparam). Objetos: casados pelo tipo e pela posição no
primeiro tique comum (o mais perto, no máximo --raio px); comparam posição, camada, vida,
rótulo, quadro e presença.

--defasagem K compara o tique t do porte com o t+K do original (ex.: 1 quando o porte
aplica a velocidade um tique antes).

Uso:
  compare.py original.jsonl porte.jsonl [--campos x,y,vx,vy] [--linhas 40] [--objetos]
"""

import argparse
import json
import sys

CAMPOS_JOGADOR = ["x", "y", "vx", "vy", "rot", "q", "cam", "vida", "arm", "arma",
                  "veneno", "lado", "inv"]
CAMPOS_OBJ = ["x", "y", "cam", "vida", "rot", "q"]


def carregar(caminho):
    meta, tiques = {}, {}
    with open(caminho) as f:
        for ln in f:
            ln = ln.strip()
            if not ln:
                continue
            d = json.loads(ln)
            if "meta" in d:
                meta = d["meta"]
                continue
            if d["t"] < 0:
                continue
            for o in d["obj"]:
                if "inv" in o:
                    o["inv"] = {str(k): v for k, v in o["inv"].items()}
            tiques[d["t"]] = d
    return meta, tiques


def jogador(q):
    for o in q["obj"]:
        if o.get("jogador"):
            return o
    return None


def casar(orig, porte, raio):
    """id do original → id do porte, pelo tipo e pela posição mais próxima."""
    livres = [o for o in porte["obj"] if not o.get("jogador")]
    par = {}
    for o in orig["obj"]:
        if o.get("jogador") or not presente(o):
            continue
        cands = [(abs(p["x"] - o["x"]) + abs(p["y"] - o["y"]), i)
                 for i, p in enumerate(livres) if p["tipo"] == o["tipo"]]
        if not cands:
            continue
        d, i = min(cands)
        if d <= raio:
            par[o["id"]] = livres.pop(i)["id"]
    return par


def presente(o):
    """Original: fora da lista dos destruídos e com o bit 0 de +0x178 (o item apanhado
    fica na célula só com 0x40); porte: `vivo`."""
    if o is None:
        return False
    if "vivo" in o:
        return bool(o["vivo"])
    return o.get("lista") != "m" and bool(o.get("fl", 1) & 1)


def fmt(v):
    if isinstance(v, dict):
        return "{" + " ".join(f"{k}:{a}/{b}" for k, (a, b) in sorted(
            v.items(), key=lambda kv: int(kv[0]))) + "}"
    return str(v)


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("original")
    ap.add_argument("porte")
    ap.add_argument("--campos", default=",".join(CAMPOS_JOGADOR),
                    help="campos do jogador a comparar")
    ap.add_argument("--defasagem", type=int, default=0,
                    help="compara porte[t] com original[t + K]")
    ap.add_argument("--linhas", type=int, default=60, help="máximo de linhas por tique")
    ap.add_argument("--tudo", action="store_true", help="lista também os tiques iguais")
    ap.add_argument("--objetos", action="store_true", help="compara também os objetos")
    ap.add_argument("--raio", type=int, default=24, help="casamento de objetos (px)")
    ap.add_argument("--json", action="store_true", help="resumo em JSON")
    a = ap.parse_args()

    mo, O = carregar(a.original)
    mp, P = carregar(a.porte)
    campos = [c for c in a.campos.split(",") if c]
    ts = sorted(t for t in P if t + a.defasagem in O)
    if not ts:
        sys.exit("nenhum tique em comum")
    if mo.get("input") is not None:
        print(f"original: fase {mo.get('fase')}, entrada {mo.get('input')!r}, "
              f"{len(O)} tiques; porte: {len(P)} tiques; defasagem {a.defasagem}")

    # entrada: as duas execuções têm de ter recebido a mesma
    for t in ts:
        if O[t + a.defasagem]["in"] != P[t]["in"] and a.defasagem == 0:
            print(f"aviso: entrada diferente no tique {t}: original {O[t]['in']:#x}, "
                  f"porte {P[t]['in']:#x}")
            break

    resumo = {c: {"difere": 0, "primeiro": None} for c in campos}
    linhas = []
    for t in ts:
        jo, jp = jogador(O[t + a.defasagem]), jogador(P[t])
        if jo is None or jp is None:
            continue
        dif = []
        for c in campos:
            vo, vp = jo.get(c), jp.get(c)
            if c == "inv" and vo is not None and vp is not None:
                ks = set(vo) | set(vp)
                dd = {k: (vo.get(k), vp.get(k)) for k in ks if vo.get(k) != vp.get(k)}
                if dd:
                    dif.append(f"inv {fmt(dd)}")
                    r = resumo[c]
                    r["difere"] += 1
                    r["primeiro"] = t if r["primeiro"] is None else r["primeiro"]
                continue
            if vo != vp:
                dif.append(f"{c} {vo}/{vp}")
                r = resumo[c]
                r["difere"] += 1
                r["primeiro"] = t if r["primeiro"] is None else r["primeiro"]
        if dif or a.tudo:
            linhas.append(f"{t:4} in={P[t]['in']:#07x} est {jo.get('est')}/{jp.get('est')}"
                          f"  ({jo['x']},{jo['y']})/({jp['x']},{jp['y']})"
                          + ("  " + "; ".join(dif) if dif else "  igual"))

    print(f"\njogador: {len(ts)} tiques comparados (original/porte)")
    for c, r in resumo.items():
        if r["difere"]:
            print(f"  {c:6} difere em {r['difere']:4} tiques, o primeiro é o {r['primeiro']}")
        else:
            print(f"  {c:6} igual")
    for ln in linhas[:a.linhas]:
        print(ln)
    if len(linhas) > a.linhas:
        print(f"  … mais {len(linhas) - a.linhas} tiques diferentes")

    if a.objetos:
        t0 = ts[0]
        par = casar(O[t0 + a.defasagem], P[t0], a.raio)
        print(f"\nobjetos: {len(par)} casados no tique {t0}")
        porte_por_id = {t: {o["id"]: o for o in P[t]["obj"]} for t in ts}
        sem_par = [o for o in O[t0 + a.defasagem]["obj"]
                   if not o.get("jogador") and presente(o) and o["id"] not in par]
        for o in sem_par:
            print(f"  sem par no porte: original id {o['id']} tipo {o['tipo']} "
                  f"({o['x']},{o['y']})")
        for oid, pid in sorted(par.items()):
            primeiro, n, tipo = None, 0, None
            por_campo = {}
            for t in ts:
                oo = next((x for x in O[t + a.defasagem]["obj"] if x["id"] == oid), None)
                pp = porte_por_id[t].get(pid)
                vivo_o = presente(oo)
                vivo_p = presente(pp)
                dif = []
                if vivo_o != vivo_p:
                    dif.append(f"presente {vivo_o}/{vivo_p}")
                elif vivo_o:
                    tipo = oo["tipo"]
                    dif = [f"{c} {oo.get(c)}/{pp.get(c)}" for c in CAMPOS_OBJ
                           if oo.get(c) != pp.get(c)]
                if dif:
                    n += 1
                    for d in dif:
                        c = d.split(" ", 1)[0]
                        por_campo[c] = por_campo.get(c, 0) + 1
                    if primeiro is None:
                        primeiro = (t, dif)
            tipo = tipo if tipo is not None else "?"
            if primeiro:
                campos_txt = ", ".join(f"{c} {k}" for c, k in sorted(por_campo.items()))
                print(f"  id {oid} (tipo {tipo}) ↔ porte {pid}: difere em {n} tiques "
                      f"({campos_txt}); primeiro {primeiro[0]}: {'; '.join(primeiro[1])}")
            else:
                print(f"  id {oid} (tipo {tipo}) ↔ porte {pid}: igual")

    if a.json:
        print(json.dumps({"jogador": resumo}, ensure_ascii=False))


if __name__ == "__main__":
    main()
