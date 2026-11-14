// SPDX-License-Identifier: GPL-3.0-or-later
//! As telas de dois jogadores sobre os dados reais (OPENHUNTED_DATA): `SEGOC`, `SHVLD` e
//! `SH2HS` (`docs/re/multijogador.md`, M15b).

use openhunted_engine::character::Character;
use openhunted_engine::devices::Device;
use openhunted_engine::shell::choose::{self, Choose};
use openhunted_engine::shell::duo::{self, DuoResults, Line, Totals};
use openhunted_engine::shell::limits::{self, Limits};
use openhunted_engine::shell::results::Choice;
use openhunted_engine::shell::{Dir, Input, Page, Placement, Reply, Screen};
use openhunted_engine::{Assets, Framebuffer, Outcome};
use openhunted_formats::{Interface, open_data};

fn assets() -> Option<Assets> {
    let Some(p) = std::env::var_os("OPENHUNTED_DATA") else {
        eprintln!("pulado: OPENHUNTED_DATA não definido");
        return None;
    };
    let gd = open_data(std::path::Path::new(&p)).unwrap();
    Some(Assets::new(gd.resources().unwrap()))
}

fn screen(a: &mut Assets, name: &str, p: Placement) -> Screen {
    let itf = Interface::parse(a.archive().get(name).unwrap()).unwrap();
    Screen::new(&itf, a, p).unwrap()
}

fn ticks(p: &mut dyn Page, n: usize) -> Option<Reply> {
    let mut snd = Vec::new();
    let mut got = None;
    for _ in 0..n {
        got = got.or(p.handle(Input::Tick, &mut snd));
    }
    got
}

fn click(p: &mut dyn Page, id: u16) -> Option<Reply> {
    let r = p.screen().rect(id).unwrap();
    let mut snd = Vec::new();
    p.handle(Input::PointerMove(r.x + r.w / 2, r.y + r.h / 2), &mut snd);
    let mut got = p.handle(Input::PointerDown, &mut snd);
    for i in 0..8 {
        if i == 1 {
            got = got.or(p.handle(Input::PointerUp, &mut snd));
        }
        got = got.or(p.handle(Input::Tick, &mut snd));
    }
    got
}

#[test]
fn segoc_picks_character_and_colour() {
    let Some(mut a) = assets() else { return };
    let s = screen(&mut a, "SEGOC.ITF", Placement::Full);
    let mut c = Choose::new(
        s,
        1,
        Device::Joystick(1),
        choose::defaults(1),
        Default::default(),
        7,
    );
    use choose::ids;
    assert_eq!(
        c.screen.frame(ids::PICK[1]).unwrap().0,
        2,
        "Garathe marcado"
    );
    assert_eq!(
        c.screen.frame(ids::BROWN).unwrap().0,
        2,
        "marrom (36) marcado"
    );
    // o retrato do Garathe acende (fade para a frente) e o do Jake fica apagado
    ticks(&mut c, 5);
    assert_eq!(c.screen.frame(ids::PORTRAIT[1]).unwrap().0, 5);
    assert_eq!(c.screen.frame(ids::PORTRAIT[0]).unwrap().0, 0);
    // o dispositivo do jogador 2: "ply2joy1" (rótulo 22)
    let mut fb = Framebuffer::new(640, 480);
    c.draw(&mut fb);
    assert_eq!(click(&mut c, ids::INPUT), Some(Reply::Devices));
    assert_eq!(click(&mut c, ids::RED), None);
    assert_eq!(c.colour(), 38);
    assert_eq!(c.screen.frame(ids::RED).unwrap().0, 2, "vermelho marcado");
    ticks(&mut c, 40);
    assert_eq!(click(&mut c, ids::PICK[0]), None);
    assert_eq!(c.character(), Character::Jake);
    assert_eq!(c.colour(), 36, "cada retrato guarda a sua cor");
    ticks(&mut c, 3);
    assert_eq!(
        c.screen.frame(ids::PORTRAIT[1]).unwrap().0,
        0,
        "o Garathe apagou"
    );
    assert!(
        c.screen.frame(ids::PORTRAIT[0]).unwrap().0 > 5,
        "o Jake acende"
    );
    assert_eq!(
        click(&mut c, ids::OK),
        Some(Reply::Chosen(Character::Jake, 36))
    );
    assert_eq!(click(&mut c, ids::CANCEL_BUTTON), Some(Reply::Close));
    let mut snd = Vec::new();
    assert_eq!(c.handle(Input::Cancel, &mut snd), Some(Reply::Close));
    // as setas andam pelos botões de `0x480428`
    c.handle(Input::Nav(Dir::Right), &mut snd);
    let p = c.screen.pointer();
    let on = ids::NAV
        .iter()
        .any(|&id| c.screen.rect(id).unwrap().w > 0 && c.screen.hit(id));
    assert!(on, "ponteiro em {p:?} sobre um botão navegável");
}

#[test]
fn shvld_steps_repeat_and_go() {
    let Some(mut a) = assets() else { return };
    let font = a.font("SLEVD.PFT").unwrap();
    let bar = a.bitmap("INVBAR01.BMP").ok();
    let s = screen(&mut a, "SHVLD.ITF", Placement::Centered);
    let mut l = Limits::new(s, font, "Silence of the Beast", bar, 2, 0);
    use limits::ids;
    assert_eq!(l.limits(), (2, 0), "abre com os limites anteriores");
    click(&mut l, ids::TIME_UP);
    click(&mut l, ids::KILLS_UP);
    click(&mut l, ids::KILLS_UP);
    click(&mut l, ids::KILLS_DOWN);
    assert_eq!(l.limits(), (3, 1));
    for _ in 0..5 {
        click(&mut l, ids::TIME_DOWN);
    }
    assert_eq!(l.limits(), (0, 1), "o tempo não fica negativo");
    // botão preso: repete com esperas 5, 4, 3, 2, 1 e depois todo quadro
    let r = l.screen.rect(ids::KILLS_UP).unwrap();
    let mut snd = Vec::new();
    l.handle(Input::PointerMove(r.x + 2, r.y + 2), &mut snd);
    l.handle(Input::PointerDown, &mut snd);
    ticks(&mut l, 1);
    assert_eq!(l.limits().1, 2);
    ticks(&mut l, 5);
    assert_eq!(l.limits().1, 2, "espera 5 quadros");
    ticks(&mut l, 1);
    assert_eq!(l.limits().1, 3);
    ticks(&mut l, 200);
    assert_eq!(l.limits().1, 99, "teto 1980 / 20");
    l.handle(Input::PointerUp, &mut snd);
    assert_eq!(
        l.shown()[1],
        99 * limits::STEP,
        "o contador rolou até o alvo"
    );
    let mut fb = Framebuffer::new(640, 480);
    l.draw(&mut fb);
    let n = l.screen.rect(ids::LEVEL_NAME).unwrap();
    let named = (n.y..n.y + n.h)
        .flat_map(|y| (n.x..n.x + n.w).map(move |x| (x, y)))
        .filter(|&(x, y)| fb.pixels[y as usize * 640 + x as usize] == limits::NAME_COLOR)
        .count();
    assert!(named > 30, "nome da fase na cor 162 ({named})");
    assert_eq!(
        click(&mut l, ids::GO),
        Some(Reply::Limits {
            minutes: 0,
            points: 99
        })
    );
    assert_eq!(click(&mut l, ids::CANCEL_BUTTON), Some(Reply::Close));
}

#[test]
fn sh2hs_fields_by_mode() {
    let Some(mut a) = assets() else { return };
    let font = a.font("SLEVD.PFT").unwrap();
    let lines = [
        Line {
            kills: 2,
            frags: 5,
            suicides: 1,
            ..Default::default()
        },
        Line {
            kills: 1,
            frags: 3,
            ..Default::default()
        },
    ];
    let totals = Totals {
        kills: 4,
        ..Default::default()
    };
    use duo::ids;
    for deathmatch in [true, false] {
        let s = screen(&mut a, "SH2HS.ITF", Placement::Full);
        let mut r = DuoResults::new(
            s,
            font.clone(),
            "Two Jakes",
            Outcome::Defeat,
            deathmatch,
            lines,
            totals,
        );
        assert!(r.screen.rect(ids::SUCCESS).is_none(), "derrota: tira 30031");
        let mut snd = Vec::new();
        r.handle(Input::Tick, &mut snd);
        assert_eq!(snd, [3093]);
        r.handle(Input::Activate, &mut snd);
        assert_eq!(r.progress(), 100, "tecla salta a contagem");
        let t = |id: u16| r.screen.text(id).map(str::to_string);
        assert_eq!(t(ids::FIRST[0]), Some("2".into()));
        assert_eq!(t(ids::FIRST[0] + 1), Some("60".into()));
        assert_eq!(t(ids::FIRST[0] + 2), Some("50".into()));
        assert_eq!(t(ids::FIRST[1] + 5), Some("100".into()), "total 0 → 100 %");
        assert_eq!(t(ids::FIRST[1] + 11), Some("30".into()));
        let (kills, killo) = if deathmatch {
            (Some("1".into()), Some("5".into()))
        } else {
            (None, None)
        };
        assert_eq!(t(ids::FIRST[0] + 9), kills, "KILLS = S");
        assert_eq!(t(ids::FIRST[0] + 10), killo, "KILLO = F");
        let mut fb = Framebuffer::new(640, 480);
        r.draw(&mut fb);
        assert_eq!(r.handle(Input::Cancel, &mut snd), None, "Esc vindo do jogo");
        r.leave_game();
        assert_eq!(
            r.handle(Input::Cancel, &mut snd),
            Some(Reply::Choice(Choice::Main))
        );
        assert_eq!(
            click(&mut r, ids::REPLAY),
            Some(Reply::Choice(Choice::Replay))
        );
        assert_eq!(click(&mut r, ids::NEXT), Some(Reply::Choice(Choice::Next)));
        assert_eq!(click(&mut r, ids::MAIN), Some(Reply::Choice(Choice::Main)));
    }
}
