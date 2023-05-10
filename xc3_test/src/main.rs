use std::{
    io::{BufReader, Cursor},
    path::Path,
};

use binrw::{BinRead, BinReaderExt};
use rayon::prelude::*;
use xc3_lib::{
    dds::{create_dds, create_mibl},
    mibl::Mibl,
    mxmd::Mxmd,
    xcb1::Xbc1,
};

fn check_wimdo<P: AsRef<Path>>(root: P) {
    globwalk::GlobWalkerBuilder::from_patterns(root, &["*.wimdo"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
            // TODO: How to validate this file?
            // TODO: The map folder is a different format?
            match Mxmd::read_le(&mut reader) {
                Ok(_) => (),
                Err(e) => println!("Error reading {path:?}: {e}"),
            }
        });
}

fn check_tex_nx_textures<P: AsRef<Path>>(root: P) {
    let folder = root.as_ref().join("chr").join("tex").join("nx").join("m");

    // TODO: the h directory doesn't have mibl footers?
    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.wismt"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let mibl = read_wismt_single_tex(&path);
            check_mibl(mibl);
        });
}

fn check_monolib_shader_textures<P: AsRef<Path>>(root: P) {
    let folder = root.as_ref().join("monolib").join("shader");

    globwalk::GlobWalkerBuilder::from_patterns(folder, &["*.{witex,witx}"])
        .build()
        .unwrap()
        .par_bridge()
        .for_each(|entry| {
            let path = entry.as_ref().unwrap().path();
            let mibl = Mibl::from_file(&path).unwrap();
            check_mibl(mibl);
        });
}

fn check_mibl(mibl: Mibl) {
    // Check that the mibl can be reconstructed from the dds.
    let dds = create_dds(&mibl).unwrap();
    let new_mibl = create_mibl(&dds).unwrap();

    // Check that the description of the image data remains unchanged.
    if mibl.footer != new_mibl.footer {
        println!("{:?} != {:?}", mibl.footer, new_mibl.footer);
    };

    // TODO: Why does this not work?
    // assert_eq!(mibl.image_data.len(), new_mibl.image_data.len());
}

fn read_wismt_single_tex<P: AsRef<Path>>(path: P) -> Mibl {
    let mut reader = BufReader::new(std::fs::File::open(path).unwrap());
    let xbc1: Xbc1 = reader.read_le().unwrap();

    let decompressed = xbc1.decompress().unwrap();
    let mut reader = Cursor::new(&decompressed);
    reader.read_le_args((decompressed.len(),)).unwrap()
}

fn main() {
    // TODO: clap for args to enable/disable different tests?
    let args: Vec<_> = std::env::args().collect();

    let root = Path::new(&args[1]);

    let start = std::time::Instant::now();

    println!("Checking chr/tex/nx/m textures ...");
    check_tex_nx_textures(root);

    println!("Checking monolib/shader textures ...");
    check_monolib_shader_textures(root);

    println!("Checking *.wimdo ...");
    check_wimdo(root);

    println!("Finished in {:?}", start.elapsed());
}