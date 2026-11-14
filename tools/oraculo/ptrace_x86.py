# SPDX-License-Identifier: GPL-3.0-or-later
"""ptrace mínimo (ctypes) para parar uma thread do Wine num endereço do Hunter.exe.

Usa só os registradores de depuração (DR0–DR3) do processador: o código do jogo não é
alterado (nada de int3). O traçador é um processo de 64 bits; o deslocamento de
`u_debugreg` é o da `struct user` do x86_64, que o kernel usa mesmo com o rastreado em
32 bits.
"""

import ctypes
import os
import signal
import struct

PTRACE_PEEKUSER = 3
PTRACE_POKEUSER = 6
PTRACE_CONT = 7
PTRACE_GETREGS = 12
PTRACE_DETACH = 17
PTRACE_SEIZE = 0x4206
PTRACE_INTERRUPT = 0x4207
PTRACE_EVENT_STOP = 128
WALL = 0x40000000  # __WALL: espera também por threads (clone)

# offsetof(struct user, u_debugreg) no x86_64
DEBUGREG_OFS = 848

_libc = ctypes.CDLL(None, use_errno=True)
_libc.ptrace.argtypes = [ctypes.c_long, ctypes.c_int, ctypes.c_void_p, ctypes.c_void_p]
_libc.ptrace.restype = ctypes.c_long


def _pt(req, tid, addr=0, data=0):
    ctypes.set_errno(0)
    r = _libc.ptrace(req, tid, ctypes.c_void_p(addr), ctypes.c_void_p(data))
    e = ctypes.get_errno()
    if r == -1 and e != 0:
        raise OSError(e, f"ptrace({req}, {tid}): {os.strerror(e)}")
    return r


class Regs(ctypes.Structure):
    """user_regs_struct do x86_64 (o rastreado de 32 bits aparece nos registradores baixos)."""
    _fields_ = [(n, ctypes.c_ulonglong) for n in (
        "r15 r14 r13 r12 rbp rbx r11 r10 r9 r8 rax rcx rdx rsi rdi orig_rax rip cs "
        "eflags rsp ss fs_base gs_base ds es fs gs").split()]


class Thread:
    """Uma thread rastreada com PTRACE_SEIZE (sem SIGSTOP de grupo)."""

    def __init__(self, tid):
        self.tid = tid
        _pt(PTRACE_SEIZE, tid, 0, 0)
        self.stopped = False

    def interrupt(self):
        """Para a thread e espera a parada (sinais pendentes são repassados)."""
        _pt(PTRACE_INTERRUPT, self.tid)
        while True:
            _, st = os.waitpid(self.tid, WALL)
            if not os.WIFSTOPPED(st):
                raise RuntimeError(f"thread {self.tid} saiu ({st:#x})")
            if (st >> 16) == PTRACE_EVENT_STOP:
                self.stopped = True
                return
            # sinal de verdade chegou antes da interrupção: entrega e continua esperando
            _pt(PTRACE_CONT, self.tid, 0, os.WSTOPSIG(st))

    def peek_dr(self, i):
        return _pt(PTRACE_PEEKUSER, self.tid, DEBUGREG_OFS + 8 * i, 0) & 0xFFFFFFFF

    def poke_dr(self, i, v):
        _pt(PTRACE_POKEUSER, self.tid, DEBUGREG_OFS + 8 * i, v)

    def set_breakpoints(self, addrs):
        """Até 4 pontos de parada de execução (DR0–DR3); a thread tem de estar parada."""
        assert len(addrs) <= 4
        self.poke_dr(7, 0)
        dr7 = 0
        for i, a in enumerate(addrs):
            self.poke_dr(i, a)
            dr7 |= 1 << (2 * i)  # Ln = 1, R/Wn = 00 (execução), LENn = 00
        self.poke_dr(7, dr7)

    def regs(self):
        r = Regs()
        _pt(PTRACE_GETREGS, self.tid, 0, ctypes.addressof(r))
        return r

    def cont(self, sig=0):
        _pt(PTRACE_CONT, self.tid, 0, sig)
        self.stopped = False

    def wait_break(self, timeout=None):
        """Continua a thread até um ponto de parada; devolve o EIP. Sinais do próprio Wine
        (SIGUSR1, SIGALRM, SIGIO…) são repassados sem parar o rastreamento."""
        import time
        t0 = time.monotonic()
        while True:
            if timeout is not None:
                pid, st = os.waitpid(self.tid, WALL | os.WNOHANG)
                if pid == 0:
                    if time.monotonic() - t0 > timeout:
                        raise TimeoutError("ponto de parada não alcançado")
                    time.sleep(0.0005)
                    continue
            else:
                _, st = os.waitpid(self.tid, WALL)
            if os.WIFEXITED(st) or os.WIFSIGNALED(st):
                raise RuntimeError(f"thread {self.tid} terminou ({st:#x})")
            sig = os.WSTOPSIG(st)
            ev = st >> 16
            if ev == PTRACE_EVENT_STOP:
                _pt(PTRACE_CONT, self.tid, 0, 0)
                continue
            if sig == signal.SIGTRAP:
                dr6 = self.peek_dr(6)
                if dr6 & 0xF:
                    self.poke_dr(6, 0)
                    self.stopped = True
                    return self.regs().rip & 0xFFFFFFFF
                _pt(PTRACE_CONT, self.tid, 0, sig)
                continue
            _pt(PTRACE_CONT, self.tid, 0, sig)

    def detach(self):
        try:
            if not self.stopped:
                self.interrupt()
            self.poke_dr(7, 0)
        except OSError:
            pass
        try:
            _pt(PTRACE_DETACH, self.tid, 0, 0)
        except OSError:
            pass


class Mem:
    """Leitura e escrita da memória do processo por /proc/<pid>/mem."""

    def __init__(self, pid):
        self.f = open(f"/proc/{pid}/mem", "r+b", buffering=0)

    def read(self, addr, n):
        self.f.seek(addr)
        return self.f.read(n)

    def write(self, addr, data):
        self.f.seek(addr)
        self.f.write(data)

    def i32(self, addr):
        return struct.unpack("<i", self.read(addr, 4))[0]

    def u32(self, addr):
        return struct.unpack("<I", self.read(addr, 4))[0]

    def w32(self, addr, v):
        self.write(addr, struct.pack("<i", v))
