//! Complete four-byte-pattern audit, partitionable by first-word range.
//! Usage: cargo run --release --example exhaustive -- TARGET START END THREADS
//! TARGET: 0=300, 1=300H, 2=S2000, 3=S2600, 4=SX. Range is decimal,
//! start inclusive/end exclusive. Only 0..65536 is a complete target sweep.
use h8_asm::{
    isa::{decode_insn, encode_insn, insn_len},
    Mode, Target,
};
use std::{
    env,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Instant,
};

fn main() {
    let args: Vec<usize> = env::args().skip(1).map(|s| s.parse().unwrap()).collect();
    assert_eq!(args.len(), 4, "TARGET START END THREADS");
    let target = [
        Target::H8_300,
        Target::H8_300H,
        Target::H8S2000,
        Target::H8S2600,
        Target::H8SX,
    ][args[0]];
    let mode = match target {
        Target::H8_300 => Mode::Normal,
        Target::H8SX => Mode::Maximum,
        _ => Mode::Advanced,
    };
    let (start, end, workers) = (args[1], args[2], args[3]);
    assert!(start < end && end <= 65536 && workers > 0);
    let next = Arc::new(AtomicUsize::new(start));
    let clock = Instant::now();
    let mut jobs = Vec::new();
    for _ in 0..workers {
        let next = next.clone();
        jobs.push(thread::spawn(move || {
            let mut counts = [0u64; 3];
            loop {
                let first = next.fetch_add(1, Ordering::Relaxed);
                if first >= end {
                    break;
                }
                // Address two is an odd-word address; no aligned u32 load is
                // assumed. Decode only the four-byte region, not its guards.
                let mut buffer = [0xa5; 8];
                buffer[2..4].copy_from_slice(&(first as u16).to_be_bytes());
                let short = decode_insn(&buffer[2..4], target, mode).ok();
                if let Some(decoded) = short {
                    assert_eq!(
                        encode_insn(decoded.insn, target, mode).unwrap(),
                        buffer[2..4]
                    );
                }
                for second in 0..=u16::MAX {
                    buffer[4..6].copy_from_slice(&second.to_be_bytes());
                    let bytes = &buffer[2..6];
                    let decoded = decode_insn(bytes, target, mode);
                    assert_eq!(
                        decoded.as_ref().ok().map(|d| d.len),
                        insn_len(bytes, target, mode),
                        "{target:?} {bytes:02x?}"
                    );
                    match decoded {
                        Ok(decoded) if decoded.len == 2 => {
                            assert_eq!(
                                Some(decoded),
                                short,
                                "suffix changed a two-byte instruction"
                            );
                            counts[1] += 1;
                        }
                        Ok(decoded) => {
                            assert_eq!(decoded.len, 4);
                            assert_eq!(
                                encode_insn(decoded.insn, target, mode).unwrap(),
                                bytes,
                                "{target:?} {bytes:02x?}"
                            );
                            counts[2] += 1;
                        }
                        Err(_) => {
                            assert!(short.is_none());
                            counts[0] += 1;
                        }
                    }
                }
            }
            counts
        }));
    }
    let mut counts = [0u64; 3];
    for job in jobs {
        for (sum, value) in counts.iter_mut().zip(job.join().unwrap()) {
            *sum += value;
        }
    }
    assert_eq!(counts.iter().sum::<u64>(), ((end - start) as u64) << 16);
    if start == 0 && end == 65536 {
        let expected = [
            [896617087, 3367239680, 31110529],
            [652133790, 3561488384, 81345122],
            [629064042, 3584557056, 81346198],
            [626901290, 3586719744, 81346262],
            [526073994, 3675258880, 93634422],
        ];
        assert_eq!(counts, expected[args[0]], "full-sweep census changed");
    }
    println!("target={target:?} mode={mode:?} start={start} end={end} rejected={} two_byte={} four_byte={} patterns={} seconds={:.3}", counts[0], counts[1], counts[2], counts.iter().sum::<u64>(), clock.elapsed().as_secs_f64());
}
