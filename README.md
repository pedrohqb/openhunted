# OpenHunted

Reimplementação livre da engine de **Hunter Hunted** (K.A.A./Dynamix, Sierra On-Line, 1996)
para Linux, em Rust com SDL3.

O objetivo é recriar o jogo original com o mesmo comportamento (física, inimigos, armas,
regras das missões), rodando nativamente, com a tela original ampliada e som funcionando.

> **É preciso o CD original.** O OpenHunted lê os dados do CD do jogo (versão Windows 95 1.00).
> Nenhum dado do jogo — imagens, sons, fases, executável — é distribuído com este projeto.

## Status

M1: formatos em andamento. Ainda não há nada jogável. O plano e o design estão em
[`docs/superpowers/`](docs/superpowers/).

## Compilar

Requer Rust estável (1.85 ou mais recente).

```sh
cargo build --release
```

## Testes

```sh
cargo test --workspace
```

Os testes sobre os dados reais do jogo só rodam quando a variável `OPENHUNTED_DATA` aponta
para os dados — a raiz do CD montado, a pasta `Hunter/` do CD ou uma imagem `.iso` do CD:

```sh
OPENHUNTED_DATA=/caminho/Hunter_Hunted.iso cargo test --workspace
```

Sem a variável, esses testes são pulados com um aviso.

## Licença

GPL-3.0-or-later. Veja [`LICENSE`](LICENSE).

Hunter Hunted é marca de seus respectivos titulares. Este projeto não tem relação com a
Sierra, a Dynamix ou seus sucessores.
