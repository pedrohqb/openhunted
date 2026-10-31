// SPDX-License-Identifier: GPL-3.0-or-later
//! Geradores de dados sintéticos para testes (feature `test-support`).

const SECTOR: usize = 2048;

/// Monta uma imagem ISO 9660 mínima com os arquivos dados (um nível de subpastas).
/// Sem Joliet, os nomes vão em maiúsculas com sufixo `;1`; com Joliet, o SVD
/// guarda os nomes originais em UCS-2 big-endian.
pub fn mkiso(files: &[(&str, &[u8])], joliet: bool) -> Vec<u8> {
    // Árvore: pastas de primeiro nível e arquivos.
    let mut dirs: Vec<String> = Vec::new();
    for (path, _) in files {
        if let Some((d, _)) = path.split_once('/') {
            if !dirs.iter().any(|x| x == d) {
                dirs.push(d.to_string());
            }
        }
    }
    let n_trees = if joliet { 2 } else { 1 };
    // Setores: 16 vazios, PVD, [SVD], terminador, depois extents de diretório.
    let desc_count = 1 + usize::from(joliet) + 1;
    let mut next = 16 + desc_count;
    // Cada árvore: raiz + uma por pasta, um setor cada.
    let mut dir_lba = vec![vec![0usize; dirs.len() + 1]; n_trees];
    for tree in dir_lba.iter_mut() {
        for slot in tree.iter_mut() {
            *slot = next;
            next += 1;
        }
    }
    let mut file_lba = Vec::new();
    for (_, data) in files {
        file_lba.push(next);
        next += data.len().div_ceil(SECTOR).max(1);
    }
    let mut img = vec![0u8; next * SECTOR];

    for (t, lbas) in dir_lba.iter().enumerate() {
        let j = t == 1;
        // Raiz.
        let mut recs = vec![
            record(&[0], lbas[0], SECTOR, true),
            record(&[1], lbas[0], SECTOR, true),
        ];
        for (i, d) in dirs.iter().enumerate() {
            recs.push(record(&encode(d, j, false), lbas[i + 1], SECTOR, true));
        }
        for (k, (path, data)) in files.iter().enumerate() {
            if !path.contains('/') {
                recs.push(record(
                    &encode(path, j, true),
                    file_lba[k],
                    data.len(),
                    false,
                ));
            }
        }
        put(&mut img, lbas[0] * SECTOR, &recs.concat());
        // Subpastas.
        for (i, d) in dirs.iter().enumerate() {
            let mut recs = vec![
                record(&[0], lbas[i + 1], SECTOR, true),
                record(&[1], lbas[0], SECTOR, true),
            ];
            for (k, (path, data)) in files.iter().enumerate() {
                if let Some((pd, name)) = path.split_once('/') {
                    if pd == d {
                        recs.push(record(
                            &encode(name, j, true),
                            file_lba[k],
                            data.len(),
                            false,
                        ));
                    }
                }
            }
            put(&mut img, lbas[i + 1] * SECTOR, &recs.concat());
        }
        // Descritor.
        let off = (16 + t) * SECTOR;
        img[off] = if j { 2 } else { 1 };
        img[off + 1..off + 6].copy_from_slice(b"CD001");
        img[off + 6] = 1;
        if j {
            img[off + 88..off + 91].copy_from_slice(b"%/E");
        }
        let root = record(&[0], lbas[0], SECTOR, true);
        img[off + 156..off + 156 + 34].copy_from_slice(&root);
    }
    let term = (16 + desc_count - 1) * SECTOR;
    img[term] = 255;
    img[term + 1..term + 6].copy_from_slice(b"CD001");
    img[term + 6] = 1;
    for (k, (_, data)) in files.iter().enumerate() {
        put(&mut img, file_lba[k] * SECTOR, data);
    }
    img
}

fn encode(name: &str, joliet: bool, is_file: bool) -> Vec<u8> {
    if joliet {
        name.encode_utf16().flat_map(u16::to_be_bytes).collect()
    } else if is_file {
        format!("{};1", name.to_uppercase()).into_bytes()
    } else {
        name.to_uppercase().into_bytes()
    }
}

fn record(name: &[u8], lba: usize, size: usize, is_dir: bool) -> Vec<u8> {
    let mut len = 33 + name.len();
    if len % 2 == 1 {
        len += 1;
    }
    let mut r = vec![0u8; len];
    r[0] = len as u8;
    r[2..6].copy_from_slice(&(lba as u32).to_le_bytes());
    r[6..10].copy_from_slice(&(lba as u32).to_be_bytes());
    r[10..14].copy_from_slice(&(size as u32).to_le_bytes());
    r[14..18].copy_from_slice(&(size as u32).to_be_bytes());
    r[25] = if is_dir { 2 } else { 0 };
    r[28] = 1;
    r[31] = 1;
    r[32] = name.len() as u8;
    r[33..33 + name.len()].copy_from_slice(name);
    r
}

fn put(img: &mut [u8], off: usize, data: &[u8]) {
    img[off..off + data.len()].copy_from_slice(data);
}

/// Um arquivo sintético: (nome, dados).
pub type SynthFile<'a> = (&'a str, &'a [u8]);
/// Um volume sintético: (nome do volume, arquivos).
pub type SynthVolume<'a> = (&'a str, &'a [SynthFile<'a>]);

/// Monta um mapa e volumes sintéticos no formato da Dynamix.
/// Devolve `(mapa, [(nome do volume, bytes)])`.
pub fn mkarchive(volumes: &[SynthVolume<'_>]) -> (Vec<u8>, Vec<(String, Vec<u8>)>) {
    let mut map = vec![1, 2, 3, 5];
    map.extend_from_slice(&(volumes.len() as u16).to_le_bytes());
    let mut out = Vec::new();
    for (vname, files) in volumes {
        map.extend_from_slice(&name13(vname));
        map.extend_from_slice(&(files.len() as u16).to_le_bytes());
        let mut vol = Vec::new();
        for (fname, data) in files.iter() {
            map.extend_from_slice(&0u32.to_le_bytes());
            map.extend_from_slice(&(vol.len() as u32).to_le_bytes());
            vol.extend_from_slice(&name13(fname));
            vol.extend_from_slice(&(data.len() as u32).to_le_bytes());
            vol.extend_from_slice(data);
        }
        out.push((vname.to_string(), vol));
    }
    (map, out)
}

fn name13(name: &str) -> [u8; 13] {
    let mut n = [0u8; 13];
    let b = name.as_bytes();
    n[..b.len().min(12)].copy_from_slice(&b[..b.len().min(12)]);
    n
}

/// Monta um recurso `BMP:` com quadros crus `(largura, altura, pixels)`.
pub fn mkbmp(frames: &[(u16, u16, Vec<u8>)]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&1000u16.to_le_bytes());
    body.extend_from_slice(&(frames.len() as u16).to_le_bytes());
    body.extend(std::iter::repeat_n(0u8, 4 * frames.len()));
    for (w, h, px) in frames {
        body.extend_from_slice(&w.to_le_bytes());
        body.extend_from_slice(&h.to_le_bytes());
        body.push(0);
        body.extend_from_slice(&(px.len() as u32).to_le_bytes());
        body.extend_from_slice(px);
    }
    let mut v = b"BMP:".to_vec();
    v.extend_from_slice(&(body.len() as u32).to_le_bytes());
    v.extend(body);
    v
}

/// Monta uma paleta RIFF `PAL ` em tons de cinza (cor i = (i, i, i)).
pub fn mkpal() -> Vec<u8> {
    let mut entries = Vec::new();
    entries.extend_from_slice(&0x0300u16.to_le_bytes());
    entries.extend_from_slice(&256u16.to_le_bytes());
    for i in 0..=255u8 {
        entries.extend_from_slice(&[i, i, i, 0]);
    }
    let mut data = b"data".to_vec();
    data.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    data.extend(entries);
    let mut v = b"RIFF".to_vec();
    v.extend_from_slice(&((data.len() + 4) as u32).to_le_bytes());
    v.extend_from_slice(b"PAL ");
    v.extend(data);
    v
}

/// Monta uma animação mínima válida: um quadro que desenha o quadro 0 do bitmap 2
/// (`oan00.bmp`), com um rótulo "parado".
pub fn mkanm() -> Vec<u8> {
    fn block(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = tag.to_vec();
        v.extend_from_slice(&(data.len() as u32).to_le_bytes());
        v.extend_from_slice(data);
        v
    }
    fn words(ws: &[i16]) -> Vec<u8> {
        ws.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
    let script = words(&[2, 2, 0, 5, 6, 0, 0, 1]);
    let mut anm = b"TB".to_vec();
    anm.extend(words(&[
        1004,
        0,
        1,
        1,
        script.len() as i16,
        22,
        17,
        20,
        55,
        66,
    ]));
    anm.extend(words(&[-7, 1, 20, 20, 0, 0]));
    anm.extend(words(&[0, 1]));
    anm.extend(script);
    let mut tag = words(&[1, 2]);
    tag.extend_from_slice(b"oan00.bmp\0");
    let mut lab = words(&[1, 1]);
    lab.extend_from_slice(b"parado\0");
    let mut v = block(b"ANM:", &anm);
    v.extend(block(b"TAG:", &tag));
    v.extend(block(b"LAB:", &lab));
    v
}

fn tagged(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut v = tag.to_vec();
    v.extend_from_slice(&(data.len() as u32).to_le_bytes());
    v.extend_from_slice(data);
    v
}

fn le16(ws: &[i32]) -> Vec<u8> {
    ws.iter().flat_map(|&w| (w as u16).to_le_bytes()).collect()
}

/// Monta um cenário `.BRD` com as caixas `(tipo, x, y, largura, altura)`.
pub fn mkbrd(boxes: &[(u16, i16, i16, u16, u16)]) -> Vec<u8> {
    let mut hed = b"Climber Border File\0".to_vec();
    hed.extend(le16(&[0, boxes.len() as i32]));
    let mut bdr = Vec::new();
    for &(k, x, y, w, h) in boxes {
        bdr.extend(le16(&[k as i32, x as i32, y as i32, w as i32, h as i32]));
    }
    let mut v = tagged(b"HED:", &hed);
    v.extend(tagged(b"BDR:", &bdr));
    v
}

/// Monta uma interface `.ITF` (versão 1103) com dois controles: um tipo 0 sem rótulo e um
/// tipo 4 (campo de edição, com 18 bytes extras) ligado ao rótulo 2 da animação.
pub fn mkitf() -> Vec<u8> {
    let mut itf = b"TB".to_vec();
    itf.extend(le16(&[1103]));
    itf.extend_from_slice(b"teste.ANM\0");
    itf.extend(le16(&[2, 0, 4]));
    itf.extend(le16(&[0, 0x80e8, 0, 10, 20, 100, 28, 1, 0xffff, -10, -20]));
    itf.extend(le16(&[4, 0x80e9, 0, 30, 40, 50, 60, 1, 2, -30, -40]));
    itf.extend_from_slice(&[0xaa; 18]);
    let mut tag = le16(&[2, 0x80e8]);
    tag.extend_from_slice(b"T_FUNDO\0");
    tag.extend(le16(&[0x80e9]));
    tag.extend_from_slice(b"T_NOME\0");
    let mut v = tagged(b"ITF:", &itf);
    v.extend(tagged(b"TAG:", &tag));
    v
}

/// Monta uma fonte `.PFT` com altura 2 e os glifos dados `(largura, pixels sem
/// preenchimento)`, começando no caractere `first_char`.
pub fn mkpft(first_char: u32, glyphs: &[(u32, Vec<u8>)]) -> Vec<u8> {
    fn ch(tag: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut v = tag.to_vec();
        v.extend_from_slice(&(data.len() as u32).to_le_bytes());
        v.extend_from_slice(data);
        v
    }
    let u32s = |ws: &[u32]| -> Vec<u8> { ws.iter().flat_map(|w| w.to_le_bytes()).collect() };
    let height = 2u32;
    let n = glyphs.len() as u32;
    let mut arr = ch(b"head", &u32s(&[n + 1, n]));
    arr.extend(ch(b"user", &vec![0u8; 12 * glyphs.len()]));
    let codes: Vec<u32> = (0..n).map(|i| first_char + i).collect();
    arr.extend(ch(b"rmap", &u32s(&codes)));
    for (w, px) in glyphs {
        let stride = (w + 3) & !3;
        let mut data = Vec::new();
        for row in 0..height {
            let start = (row * w) as usize;
            data.extend_from_slice(&px[start..start + *w as usize]);
            data.extend(std::iter::repeat_n(0u8, (stride - w) as usize));
        }
        let mut bmp = ch(b"head", &u32s(&[1, *w, height, 8, 1]));
        bmp.extend(ch(b"data", &data));
        // Como nos dados originais, o tamanho declarado do PBMP é 4 a menos que o real.
        let mut p = b"PBMP".to_vec();
        p.extend_from_slice(&(bmp.len() as u32 - 4).to_le_bytes());
        p.extend(bmp);
        arr.extend(p);
    }
    let mut body = ch(
        b"head",
        &u32s(&[
            1,
            69,
            0,
            n,
            height,
            height + 1,
            255,
            0,
            height,
            65536,
            65536,
            0,
        ]),
    );
    body.extend(ch(b"PBMA", &arr));
    ch(b"PFNT", &body)
}

/// Monta um `hunter.hls` sintético: índice no formato do `RESOURCE.MAP` seguido do volume,
/// com offsets relativos ao fim do índice.
pub fn mkhls(files: &[SynthFile<'_>]) -> Vec<u8> {
    let (mut map, vols) = mkarchive(&[("RESOURCE.001", files)]);
    map.extend_from_slice(&vols[0].1);
    map
}

/// Monta uma fase `.LEV` versão 1006 com grade 2×1×1, sem objetos e com as duas células vazias.
pub fn mklev() -> Vec<u8> {
    let body = le16(&[1013, 0, 0, 0]);
    let mut lev = Vec::new();
    for s in ["T_J_001.LEV", "WORLD.PAL"] {
        lev.extend_from_slice(s.as_bytes());
        lev.push(0);
    }
    lev.extend(le16(&[1006]));
    for s in ["Teste", "Chegue à saída.", "notas"] {
        lev.extend(s.bytes().map(|b| if b.is_ascii() { b } else { b'?' }));
        lev.push(0);
    }
    lev.extend(le16(&[0, 2, 1, 1, 1, 36]));
    let mut v = tagged(b"LEV:", &lev);
    v.extend_from_slice(&body);
    v
}
