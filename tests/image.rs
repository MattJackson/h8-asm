use h8_asm::{
    find, find_free_space, insert, read_u16, read_u32, read_u8, write, CommandTable, ImageError,
    Needle, TableError,
};

#[test]
fn exact_big_endian_and_masked_searches() {
    let image = [0x12, 0x34, 0x56, 0x78, 0x12, 0x34];
    assert_eq!(find(&image, Needle::Word(0x12345678), 0), Some(0));
    assert_eq!(find(&image, Needle::Bytes(&[0x12, 0x34]), 1), Some(4));
    assert_eq!(find(&image, Needle::Bytes(&[]), 0), None);
    assert_eq!(find(&image, Needle::Bytes(&[0]), usize::MAX), None);
    assert_eq!(find(&image, Needle::Bytes(&[0; 7]), 0), None);
    let pattern = [(0x1200, 0xff00), (0x0078, 0x00ff)];
    assert_eq!(find(&image, Needle::Masked(&pattern), 0), Some(0));
    assert_eq!(
        find(&image, Needle::Masked(&[(0x1234, 0xffff)]), 1),
        Some(4)
    );
    assert_eq!(find(&image, Needle::Masked(&pattern), 2), None);
    assert_eq!(find(&image, Needle::Masked(&[]), 0), None);
    assert_eq!(find(&[], Needle::Masked(&pattern), 0), None);
    assert_eq!(find(&image, Needle::Masked(&pattern), usize::MAX), None);
}

#[test]
fn free_space_alignment_and_bounds() {
    let image = [0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0];
    assert_eq!(find_free_space(&image, 3, 4, 1), Some(4));
    assert_eq!(
        find(&image, Needle::FreeRun { len: 4, align: 2 }, 0),
        Some(2)
    );
    assert_eq!(find_free_space(&image, 4, 4, 1), None);
    assert_eq!(find_free_space(&image, 9, 1, 0), None);
    assert_eq!(find_free_space(&image, 0, 1, 0), None);
    assert_eq!(find_free_space(&image, 1, 0, 0), None);
    assert_eq!(find_free_space(&image, 1, 2, usize::MAX), None);
    assert_eq!(find_free_space(&image, 1, 1, usize::MAX), None);
    assert_eq!(find_free_space(&image, 3, 3, 0), Some(3));
}

#[test]
fn checked_reads_and_transactional_writes() {
    let image = [0x12, 0x34, 0x56, 0x78];
    assert_eq!(read_u8(&image, 3), Some(0x78));
    assert_eq!(read_u16(&image, 1), Some(0x3456));
    assert_eq!(read_u32(&image, 0), Some(0x12345678));
    assert_eq!(read_u8(&image, 4), None);
    for at in [3, 4, usize::MAX] {
        assert_eq!(read_u16(&image, at), None);
    }
    for at in [1, 4, usize::MAX] {
        assert_eq!(read_u32(&image, at), None);
    }
    let mut image = [0xff; 8];
    assert_eq!(insert(&mut image, 2, &[0x12, 0x34]), Ok(2));
    let previous = image;
    assert_eq!(insert(&mut image, 1, &[0, 0]), Err(ImageError::NotFree));
    assert_eq!(insert(&mut image, 7, &[0, 0]), Err(ImageError::OutOfBounds));
    assert_eq!(
        insert(&mut image, usize::MAX, &[0]),
        Err(ImageError::OutOfBounds)
    );
    assert_eq!(write(&mut image, 7, &[0, 0]), Err(ImageError::OutOfBounds));
    assert_eq!(
        write(&mut image, usize::MAX, &[0]),
        Err(ImageError::OutOfBounds)
    );
    assert_eq!(image, previous);
    write(&mut image, 2, &[0xab, 0xcd]).unwrap();
    write(&mut image, 8, &[]).unwrap();
    assert_eq!(&image[2..4], &[0xab, 0xcd]);
}

fn table() -> CommandTable {
    CommandTable {
        base: 0,
        stride: 8,
        opcode_off: 0,
        flags_off: 1,
        handler_off: 4,
        term_flag: 0xff,
        max_records: 3,
    }
}

#[test]
fn command_table_geometry_pointers_and_atomic_replacement() {
    let table = table();
    let mut image = [
        1, 0, 0, 0, 0x12, 0x34, 0x56, 0x78, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0xff, 0, 0, 0, 0, 0, 0,
    ];
    assert_eq!(table.walk(&image).unwrap().len(), 2);
    assert_eq!(table.find(&image, 1).unwrap().handler, 0x12345678);
    assert_eq!(table.find(&image, 2).unwrap().off, 8);
    assert_eq!(table.find(&image, 3), Err(TableError::NotFound));
    table.replace(&mut image, 2, 0x87654320, Some(2)).unwrap();
    assert_eq!(&image[12..16], &[0x87, 0x65, 0x43, 0x20]);
    assert_eq!(image[9], 2);
    assert_eq!(image[7], 0x78);
    table.replace(&mut image, 1, 4, None).unwrap();
    assert_eq!(image[1], 0);
    let old = image;
    assert_eq!(
        table.replace(&mut image, 3, 0, Some(0)),
        Err(TableError::NotFound)
    );
    assert_eq!(image, old);
    for invalid in [
        CommandTable { stride: 0, ..table },
        CommandTable {
            max_records: 0,
            ..table
        },
        CommandTable {
            opcode_off: 8,
            ..table
        },
        CommandTable {
            flags_off: 8,
            ..table
        },
        CommandTable {
            handler_off: usize::MAX,
            ..table
        },
        CommandTable {
            handler_off: 5,
            ..table
        },
        CommandTable {
            opcode_off: 1,
            ..table
        },
        CommandTable {
            opcode_off: 4,
            ..table
        },
        CommandTable {
            flags_off: 7,
            ..table
        },
    ] {
        assert_eq!(invalid.walk(&image), Err(TableError::InvalidGeometry));
    }
    assert_eq!(
        CommandTable {
            base: usize::MAX,
            ..table
        }
        .walk(&image),
        Err(TableError::OutOfBounds)
    );
    assert_eq!(table.walk(&image[..7]), Err(TableError::OutOfBounds));
    assert_eq!(table.walk(&image[..16]), Err(TableError::OutOfBounds));
    assert_eq!(
        CommandTable {
            max_records: 2,
            ..table
        }
        .walk(&image),
        Err(TableError::MissingTerminator)
    );
    assert_eq!(
        table.replace(&mut image[..16], 1, 0, None),
        Err(TableError::OutOfBounds)
    );
    assert_eq!(image, old);
}

#[test]
fn multiword_masked_search_preserves_each_word_and_nonzero_start() {
    let image = [0, 0, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc];
    let needle = [(0x1230, 0xfff0), (0x5078, 0xf0ff), (0x9abc, 0xffff)];
    assert_eq!(find(&image, Needle::Masked(&needle), 1), Some(2));
    assert_eq!(find(&image, Needle::Masked(&needle), 3), None);
}
