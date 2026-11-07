// SPDX-License-Identifier: GPL-3.0-or-later
//! Entidade: um objeto da fase em jogo, com os campos do registro original que os
//! comportamentos usam (`docs/re/objetos.md`, `docs/re/logica-fase.md`).

use openhunted_formats::{Animation, Border, LevelObject};

use crate::behaviour::{
    barrel::BarrelState, bruiser::BruiserState, bullet::BulletState, casing::CasingState,
    door::DoorState, enemy_shot::EnemyShotState, exit::ExitState, explosion::ExplosionState,
    fence::FenceState, fire::FireState, graffiti::GraffitiState, hint::HintState,
    hop_mine::HopMineState, laser::LaserState, laser_eye::LaserEyeState, mine::MineState,
    mine_generator::MineGeneratorState, pickup::PickupState, security_robot::SecurityRobotState,
    sentinel::SentinelState, teleporter::TeleporterState, zone::ZoneState,
};
use crate::objects::flags;
use crate::physics::Rect;
use crate::scene::{Layer, Sprite};
use crate::{AnimPlayer, Assets};

/// Tipo de forma "caixa" nos BRD.
pub const BOX: u16 = 2;
/// Grafite: texto do `GRAFFITI.RES` pintado com o `OGF.ANM` como fonte
/// (`docs/re/objetos-tutorial.md`).
pub const GRAFFITI: u32 = 6;

/// Tipos com comportamento nesta fatia.
pub const EXIT: u32 = 5;
pub const HINT: u32 = 30;
pub const HEALTH_ITEM: u32 = 32;
pub const ZONE: u32 = 73;
pub const ARMOR_ITEM: u32 = 79;
pub const EXPLOSION: u32 = 86;
pub const MINE: u32 = 102;
pub const BULLET: u32 = 19;
pub const CASING: u32 = 22;
pub const CAR_PIECE: u32 = 9;
pub const BARREL: u32 = 28;
pub const LASER: u32 = 36;
pub const FENCE: u32 = 40;
pub const FIRE: u32 = 42;
pub const TELEPORTER: u32 = 46;
pub const LASER_EYE: u32 = 104;
/// Inimigos da M7a: Bruiser (7), robô de segurança `OSR` (8), sentinela `OSE` (10), mina
/// saltadora `OHS` (11) e gerador de minas `OHG` (103).
pub const BRUISER: u32 = 7;
pub const SECURITY_ROBOT: u32 = 8;
pub const SENTINEL: u32 = 10;
pub const HOP_MINE: u32 = 11;
pub const MINE_GENERATOR: u32 = 103;
/// Hostis da M10: Morte (3), Camaleão (4), dardo (29), catapulta (41), gerador de dardos
/// (87), formigueiro (88), Manta (89), Anticorpo (105), Entidade (107) e formiga (109).
pub const DEATH: u32 = 3;
pub const CHAMELEON: u32 = 4;
pub const DART: u32 = 29;
pub const CATAPULT: u32 = 41;
pub const DART_GENERATOR: u32 = 87;
pub const ANTHILL: u32 = 88;
pub const MANTA: u32 = 89;
pub const ANTIBODY: u32 = 105;
pub const THE_ENTITY: u32 = 107;
pub const ANT: u32 = 109;
/// Inimigos cujo quadro o comportamento escolhe (sprite congelado); os tiros inimigos não
/// param neles.
pub const ENEMIES: [u32; 14] = [
    BRUISER,
    SECURITY_ROBOT,
    SENTINEL,
    HOP_MINE,
    MINE_GENERATOR,
    DEATH,
    CHAMELEON,
    CATAPULT,
    DART_GENERATOR,
    ANTHILL,
    MANTA,
    ANTIBODY,
    THE_ENTITY,
    ANT,
];
/// Paredes e portas `OH` (tipos 50–52).
pub const DOORS: std::ops::RangeInclusive<u32> = 50..=52;
/// Itens de coleta genérica tratados: armas e munição (12–14, 16–18, 26, 27), vida e comida
/// (32, 34), veneno (33, 35), antídoto (37), armaduras (76–79), chaves (62–67), bandeiras
/// (80–85) e as armas do Garathe: clava (15), facas (25), chicote (44, 45).
pub const PICKUPS: [u32; 33] = [
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    25,
    44,
    45,
    26,
    27,
    HEALTH_ITEM,
    33,
    34,
    35,
    37,
    62,
    63,
    64,
    65,
    66,
    67,
    76,
    77,
    78,
    ARMOR_ITEM,
    80,
    81,
    82,
    83,
    84,
    85,
];
/// Projéteis das armas 3–6 do Jake.
pub const PELLET: u32 = 20;
pub const ROCKET: u32 = 21;
pub const SHELL: u32 = 23;
pub const GRENADE: u32 = 26;
pub const STAR: u32 = 27;
/// Projéteis do Garathe: faca (25) e golpe do chicote (108).
pub const KNIFE: u32 = 25;
pub const WHIP_SHOT: u32 = 108;
/// Pistão e interruptor.
pub const PISTON: u32 = 39;
pub const SWITCH: u32 = 74;

/// Estado por tipo.
#[derive(Debug, Clone)]
pub enum Behaviour {
    Static,
    Zone(ZoneState),
    Hint(HintState),
    Pickup(PickupState),
    Mine(MineState),
    Exit(ExitState),
    Explosion(ExplosionState),
    Bullet(BulletState),
    Casing(CasingState),
    LaserEye(LaserEyeState),
    Laser(LaserState),
    Barrel(BarrelState),
    Fire(FireState),
    Fence(FenceState),
    Graffiti(GraffitiState),
    CarPiece,
    Teleporter(TeleporterState),
    Door(DoorState),
    Pellet(crate::behaviour::pellet::PelletState),
    Rocket(crate::behaviour::rocket::RocketState),
    Star(crate::behaviour::star::StarState),
    Grenade(crate::behaviour::grenade::GrenadeState),
    Piston(crate::behaviour::piston::PistonState),
    Switch(crate::behaviour::switch::SwitchState),
    Sentinel(SentinelState),
    HopMine(HopMineState),
    MineGenerator(MineGeneratorState),
    SecurityRobot(Box<SecurityRobotState>),
    Bruiser(Box<BruiserState>),
    EnemyShot(EnemyShotState),
    Death(Box<crate::behaviour::death::DeathState>),
    Chameleon(Box<crate::behaviour::chameleon::ChameleonState>),
    Flyer(Box<crate::behaviour::flyer::FlyerState>),
    DartGenerator(crate::behaviour::dart_generator::DartGeneratorState),
    Catapult(crate::behaviour::catapult::CatapultState),
    TheEntity(crate::behaviour::the_entity::TheEntityState),
    Anthill(crate::behaviour::ant::AnthillState),
    Ant(crate::behaviour::ant::AntState),
    /// Personagem da IA (M11): corpo do jogador e cérebro.
    AiPlayer(Box<crate::ai::AiPlayer>),
    /// Cadáver clonado de um jogador da disputa (`0x433d80`), com a tabela de cor dele.
    Corpse(i32),
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub kind: u32,
    pub x: i32,
    pub y: i32,
    /// Sinalizadores `+0x178`.
    pub flags: i32,
    /// Camada `+0x17c`.
    pub layer: i32,
    /// Id de gatilho `+0x118`.
    pub id: i32,
    pub values: Vec<i32>,
    pub triggers: Vec<(i32, Option<i32>)>,
    /// Bits de objetivo `+0x1ac` (1 segredo, 2 acionar, 4 destruir, 8 sobreviver).
    pub objective: i32,
    pub health: i32,
    pub armor: i32,
    /// Caixas de colisão relativas à posição (BRD, ou a caixa própria do tipo).
    pub boxes: Vec<Rect>,
    /// As caixas vêm de um BRD (`+0x208`): só essas barram alguém; as caixas próprias das
    /// zonas, dicas e saída são só de contato.
    pub solid_shapes: bool,
    /// Bit "ligado" (0x40 de `+0x178`): o que as listas de gatilhos leem.
    pub on: bool,
    /// `false` = removida do jogo (apanhada, destruída).
    pub alive: bool,
    pub sprite: Option<Sprite>,
    pub behaviour: Behaviour,
}

impl Entity {
    /// Caixa envolvente das formas, em px da fase.
    pub fn bounds(&self) -> Option<Rect> {
        let x0 = self.boxes.iter().map(|b| b.x).min()?;
        let y0 = self.boxes.iter().map(|b| b.y).min()?;
        let x1 = self.boxes.iter().map(|b| b.x + b.w).max()?;
        let y1 = self.boxes.iter().map(|b| b.y + b.h).max()?;
        Some(Rect {
            x: self.x.saturating_add(x0),
            y: self.y.saturating_add(y0),
            w: x1 - x0,
            h: y1 - y0,
        })
    }

    /// Centro da caixa (ou a posição, sem caixa).
    pub fn center(&self) -> (i32, i32) {
        match self.bounds() {
            Some(b) => (b.x + b.w / 2, b.y + b.h / 2),
            None => (self.x, self.y),
        }
    }

    /// Retângulo do quadro atual do sprite, em px da fase (escala aplicada).
    pub fn sprite_rect(&self) -> Option<Rect> {
        let sp = self.sprite.as_ref()?;
        let f = sp.anim.frames.get(sp.player.frame())?;
        let (num, den) = sp.scale;
        let sc = |v: i32| (i64::from(v) * i64::from(num) / i64::from(den.max(1))) as i32;
        Some(Rect {
            x: self.x,
            y: self.y,
            w: sc(i32::from(f.width)),
            h: sc(i32::from(f.height)),
        })
    }

    pub fn present(&self) -> bool {
        self.alive
    }

    /// Desenhado neste tique: vivo e não escondido pelos gatilhos (fogo, grafite). O
    /// Camaleão invisível continua desenhado: o limiar da tabela de cor apaga o corpo
    /// (`crate::tint::chameleon`).
    pub fn visible(&self) -> bool {
        self.alive
            && match &self.behaviour {
                Behaviour::Fire(s) => s.visible,
                Behaviour::Graffiti(s) => s.visible,
                Behaviour::Death(s) => !s.hidden(),
                _ => true,
            }
    }
}

/// Escala de desenho de um objeto (`0x00434655`): os dois últimos campos do objeto (A, B)
/// são o tamanho-alvo; se A ≥ B, escala = A / largura do quadro de referência, senão
/// B / altura. Valores ausentes ou inválidos dão escala 1.
pub fn object_scale(a: i32, b: i32, ref_w: i32, ref_h: i32) -> (i32, i32) {
    if a > 0 && ref_w > 0 && a >= b {
        (a, ref_w)
    } else if b > 0 && ref_h > 0 && b > a {
        (b, ref_h)
    } else {
        (1, 1)
    }
}

/// Quadro de referência da escala de um objeto e o jogador da animação que ele mostra
/// (`0x004358b3`): com rótulo inicial (`start_label`), o primeiro quadro desse rótulo (o
/// original devolve o quadro 0 se o rótulo não existe); sem ele, o quadro 0. O terceiro
/// valor diz se o sprite fica parado (o comportamento escolhe os quadros: grafite, olho-laser,
/// barril, porta, cerca).
pub fn object_start(o: &LevelObject, a: &Animation) -> (usize, AnimPlayer, bool) {
    let n = a.frames.len();
    let frozen = matches!(
        o.kind,
        GRAFFITI | LASER_EYE | BARREL | FENCE | TELEPORTER | PISTON | SWITCH
    ) || DOORS.contains(&o.kind)
        || ENEMIES.contains(&o.kind);
    let state = o.fields.get(5).copied().unwrap_or(0);
    let label = match o.kind {
        // A saída força o rótulo 8 (`0x0040e393`).
        EXIT => Some(8),
        // Barril: rótulo = estado (3 se a fase deu 3, senão 1); olho e cerca: rótulo 1;
        // teletransporte: rótulo 2 (`obo01`, parado); peça do carro: rótulo 1 [ruling: sem
        // campanha, a peça é a primeira].
        BARREL => Some(if state == 3 { 3 } else { 1 }),
        LASER_EYE | FENCE | CAR_PIECE | PISTON => Some(1),
        // chaves e bandeiras: rótulo = tipo (`0x40f214`, `0x40b2c8`); estrela no chão: 4
        62..=67 | 80..=85 => Some(o.kind as u16),
        STAR => Some(4),
        TELEPORTER => Some(crate::behaviour::teleporter::LABEL_IDLE),
        GRAFFITI => None,
        _ => o.start_label.and_then(|l| u16::try_from(l).ok()),
    };
    if let Some(id) = label {
        if let Some(p) = AnimPlayer::at_label(a, id) {
            let f = a
                .labels
                .iter()
                .find(|l| l.id == id)
                .map_or(0, |l| l.frame as usize);
            return (if f < n { f } else { 0 }, p, frozen);
        }
    }
    (0, AnimPlayer::from_frame(a, 0), frozen)
}

/// Caixas (tipo 2) de um BRD, relativas à posição do objeto.
pub fn border_boxes(b: &Border) -> Vec<Rect> {
    b.boxes
        .iter()
        .filter(|x| x.kind == BOX)
        .map(|x| Rect {
            x: i32::from(x.x),
            y: i32::from(x.y),
            w: i32::from(x.width),
            h: i32::from(x.height),
        })
        .collect()
}

/// Monta a entidade de um objeto da fase (que não seja o Jake). Problemas de recurso vão
/// para `warnings`; a entidade é criada mesmo assim (sem sprite ou sem caixas).
pub fn from_object(o: &LevelObject, assets: &mut Assets, warnings: &mut Vec<String>) -> Entity {
    let field = |i: usize| o.fields.get(i).copied().unwrap_or(0);
    let value = |i: usize| o.values.get(i).copied().unwrap_or(0);
    let mut sprite = None;
    let mut frame_box = None;
    if !o.names[0].is_empty() {
        match assets.animation(&o.names[0]) {
            Ok(a) => {
                let n = o.fields.len();
                let (ta, tb) = if n >= 2 {
                    (o.fields[n - 2], o.fields[n - 1])
                } else {
                    (0, 0)
                };
                let (ref_frame, player, frozen) = object_start(o, &a);
                let (rw, rh) = a
                    .frames
                    .get(ref_frame)
                    .map_or((0, 0), |f| (i32::from(f.width), i32::from(f.height)));
                let scale = object_scale(ta, tb, rw, rh);
                let sc =
                    |v: i32| (i64::from(v) * i64::from(scale.0) / i64::from(scale.1.max(1))) as i32;
                frame_box = Some(Rect {
                    x: 0,
                    y: 0,
                    w: sc(rw),
                    h: sc(rh),
                });
                sprite = Some(Sprite {
                    layer: Layer::Object,
                    anim: a,
                    player,
                    x: o.x,
                    y: o.y,
                    frozen,
                    scale,
                    mirror: false,
                });
            }
            Err(e) => warnings.push(format!("objeto tipo {} ({}): {e}", o.kind, o.names[0])),
        }
    }
    let mut boxes = Vec::new();
    let mut solid_shapes = false;
    if !o.names[1].is_empty() {
        match assets.archive().get(&o.names[1]).and_then(Border::parse) {
            Ok(b) => {
                boxes = border_boxes(&b);
                solid_shapes = !boxes.is_empty();
            }
            Err(e) => warnings.push(format!("objeto tipo {} ({}): {e}", o.kind, o.names[1])),
        }
    }
    let own_box = |w: i32, h: i32| Rect { x: 0, y: 0, w, h };
    let behaviour = match o.kind {
        ZONE => {
            let w = if value(0) > 0 { value(0) } else { 32 };
            let h = if value(1) > 0 { value(1) } else { 16 };
            boxes = vec![own_box(w, h)];
            solid_shapes = false;
            Behaviour::Zone(ZoneState::default())
        }
        HINT => {
            boxes = vec![own_box(32, 32)];
            solid_shapes = false;
            Behaviour::Hint(HintState::default())
        }
        EXIT => {
            // A caixa da saída é o retângulo do quadro (`+0x20c`, `0x0040e393`).
            if let Some(fb) = frame_box {
                boxes = vec![fb];
                solid_shapes = false;
            }
            Behaviour::Exit(ExitState::default())
        }
        // a granada só é coletável no estado 1 (`0x40d754`); 0 e 2 são a granada lançada
        26 if field(5) != 1 => Behaviour::Static,
        k if PICKUPS.contains(&k) => Behaviour::Pickup(PickupState::default()),
        MINE => Behaviour::Mine(MineState::new(value(0))),
        LASER_EYE => Behaviour::LaserEye(LaserEyeState::new(&o.values)),
        BARREL => {
            let rolling = assets
                .archive()
                .get("OOB02.BRD")
                .and_then(Border::parse)
                .map(|b| border_boxes(&b))
                .unwrap_or_default();
            let st = BarrelState::new(field(5), rolling);
            if st.state == crate::behaviour::barrel::ROLLING && !st.rolling_boxes.is_empty() {
                boxes = st.rolling_boxes.clone();
                solid_shapes = true;
            }
            Behaviour::Barrel(st)
        }
        FIRE => {
            let (pos, size, fh) = match &sprite {
                Some(sp) => {
                    let r = frame_box.unwrap_or(Rect {
                        x: 0,
                        y: 0,
                        w: 0,
                        h: 0,
                    });
                    let fh = sp.anim.frames.first().map_or(1, |f| i32::from(f.height));
                    ((o.x, o.y), (r.w, r.h), fh)
                }
                None => ((o.x, o.y), (0, 0), 1),
            };
            let st = FireState::new(&o.values, pos, size, fh);
            let r = st.rect();
            boxes = vec![own_box(r.w, r.h)];
            solid_shapes = false;
            Behaviour::Fire(st)
        }
        FENCE => Behaviour::Fence(FenceState::from_values(&o.values)),
        GRAFFITI => {
            let lines = assets
                .archive()
                .get("GRAFFITI.RES")
                .map(openhunted_formats::parse_text_list)
                .unwrap_or_default();
            let text = crate::behaviour::graffiti::line(&lines, value(0));
            match &sprite {
                Some(sp) => {
                    let st = GraffitiState::new(&text, value(1), &sp.anim);
                    boxes = vec![own_box(st.width, st.scale.0)];
                    solid_shapes = false;
                    Behaviour::Graffiti(st)
                }
                None => Behaviour::Static,
            }
        }
        CAR_PIECE => Behaviour::CarPiece,
        PISTON => {
            // caixa = retângulo do quadro (só contato, `|= 0x28`)
            if let Some(fb) = frame_box {
                boxes = vec![fb];
                solid_shapes = false;
            }
            Behaviour::Piston(crate::behaviour::piston::PistonState::default())
        }
        SWITCH => {
            if let Some(fb) = frame_box {
                boxes = vec![fb];
                solid_shapes = false;
            }
            Behaviour::Switch(crate::behaviour::switch::SwitchState::default())
        }
        SENTINEL => Behaviour::Sentinel(SentinelState::new(&o.values)),
        HOP_MINE => Behaviour::HopMine(HopMineState::default()),
        MINE_GENERATOR => Behaviour::MineGenerator(MineGeneratorState::new(value(0))),
        SECURITY_ROBOT => Behaviour::SecurityRobot(Box::new(SecurityRobotState::new(
            &o.values,
            field(5),
            o.x.wrapping_mul(31).wrapping_add(o.y),
        ))),
        BRUISER => Behaviour::Bruiser(Box::new(BruiserState::new(&o.values))),
        DEATH => Behaviour::Death(Box::default()),
        CHAMELEON => Behaviour::Chameleon(Box::new(
            crate::behaviour::chameleon::ChameleonState::new(value(0)),
        )),
        MANTA | ANTIBODY => Behaviour::Flyer(Box::new(crate::behaviour::flyer::FlyerState::new(
            o.kind,
            &o.values,
            (o.x, o.y),
            o.x.wrapping_mul(31).wrapping_add(o.y),
        ))),
        DART_GENERATOR => Behaviour::DartGenerator(
            crate::behaviour::dart_generator::DartGeneratorState::new(&o.values),
        ),
        CATAPULT => {
            // sem BRD: a caixa é o retângulo do quadro, só de contato
            if let Some(fb) = frame_box {
                boxes = vec![fb];
                solid_shapes = false;
            }
            Behaviour::Catapult(crate::behaviour::catapult::CatapultState::new(&o.values))
        }
        THE_ENTITY => {
            Behaviour::TheEntity(crate::behaviour::the_entity::TheEntityState::new(&o.values))
        }
        ANTHILL => Behaviour::Anthill(crate::behaviour::ant::AnthillState::new(
            &o.values,
            field(5),
        )),
        ANT => Behaviour::Ant(crate::behaviour::ant::AntState::new(value(0), false)),
        TELEPORTER => Behaviour::Teleporter(TeleporterState::default()),
        k if DOORS.contains(&k) => {
            if boxes.is_empty() {
                let label = o.start_label.unwrap_or(0);
                let name = crate::behaviour::door::border_name(k, label);
                if let Ok(b) = assets.archive().get(&name).and_then(Border::parse) {
                    boxes = border_boxes(&b);
                    solid_shapes = !boxes.is_empty();
                }
            }
            Behaviour::Door(DoorState::new(field(5), &o.values))
        }
        _ => Behaviour::Static,
    };

    let mut health = field(14);
    if let Behaviour::Mine(m) = &behaviour {
        if health == 0 {
            health = m.variant().health;
        }
    }
    if health == 0 {
        health = match o.kind {
            SENTINEL => crate::behaviour::sentinel::HEALTH,
            HOP_MINE => crate::behaviour::hop_mine::HEALTH,
            BRUISER => crate::behaviour::bruiser::HEALTH,
            DEATH => crate::behaviour::death::HEALTH,
            CHAMELEON => match &behaviour {
                Behaviour::Chameleon(c) => c.variant().health,
                _ => 0,
            },
            MANTA | ANTIBODY => crate::behaviour::flyer::HEALTH,
            DART_GENERATOR => crate::behaviour::dart_generator::health(&o.values),
            ANT => crate::behaviour::ant::HEALTH,
            MINE_GENERATOR => match &behaviour {
                Behaviour::MineGenerator(g) => g.variant().health,
                _ => 0,
            },
            _ => 0,
        };
    }
    let mut f = flags(o);
    match o.kind {
        // `+0x178 |= 0x4803` (oculta: 0x2 a tira das colisões)
        DEATH => f |= 0x4803,
        // `+0x178 = 0x81 | 0x4000` e o leitor liga 0x80000
        MANTA | ANTIBODY => f = 0x80000 | 0x4000 | 0x81,
        _ => {}
    }
    if DOORS.contains(&o.kind) {
        // `|= 0x89`, `&= ~0x40`; aberta (estado 4) é só contato
        f = (f | 0x89) & !0x40;
        if let Behaviour::Door(d) = &behaviour {
            if d.state == crate::behaviour::door::OPEN {
                f |= 0x20;
            }
        }
    }
    Entity {
        kind: o.kind,
        x: o.x,
        y: o.y,
        flags: f,
        layer: field(4),
        id: field(18),
        values: o.values.clone(),
        triggers: o.triggers.clone(),
        objective: field(16),
        health,
        armor: field(15),
        boxes,
        solid_shapes,
        on: match &behaviour {
            Behaviour::Door(d) => d.state > crate::behaviour::door::CLOSING,
            _ => f & 0x40 != 0 || (matches!(o.kind, EXIT | SWITCH) && value(0) != 0),
        },
        alive: true,
        sprite,
        behaviour,
    }
}
