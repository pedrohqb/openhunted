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
