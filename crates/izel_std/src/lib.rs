pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }

    #[test]
    fn std_iter_exposes_full_combinator_surface() {
        let iter_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std/iter.iz");
        let src = fs::read_to_string(&iter_path).expect("failed to read std::iter fixture");

        let required = [
            "forge map<",
            "forge filter<",
            "forge filter_map<",
            "forge flat_map<",
            "forge flatten<",
            "forge fold<",
            "forge scan<",
            "forge take<",
            "forge skip<",
            "forge take_while<",
            "forge skip_while<",
            "forge zip<",
            "forge enumerate<",
            "forge chain<",
            "forge peekable<",
            "forge cloned<",
            "forge collect<",
            "forge count<",
            "forge sum<",
            "forge product<",
            "forge min<",
            "forge max<",
            "forge any<",
            "forge all<",
            "forge find<",
            "forge position<",
            "forge partition<",
            "forge chunks<",
            "forge windows<",
        ];

        for symbol in required {
            assert!(
                src.contains(symbol),
                "missing std::iter combinator declaration: {}",
                symbol
            );
        }
    }

    #[test]
    fn std_witness_exposes_builtin_surface() {
        let witness_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std/witness.iz");
        let src = fs::read_to_string(&witness_path).expect("failed to read std::witness fixture");

        let required = [
            "shape NonZero<",
            "shape InBounds<",
            "shape Sorted<",
            "forge new(value: T) -> ?NonZero<T>",
            "forge assert(value: T) -> NonZero<T> !panic",
            "forge check_index(&~self, idx: usize) -> ?InBounds<usize>",
            "forge into_sorted(self) -> Sorted<Vec<T>>",
        ];

        for symbol in required {
            assert!(
                src.contains(symbol),
                "missing std::witness built-in declaration: {}",
                symbol
            );
        }
    }

    #[test]
    fn std_concurrency_exposes_thread_and_channel_surface() {
        let thread_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std/thread.iz");
        let sync_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std/sync.iz");
        let atomic_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std/atomic.iz");
        let async_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std/async.iz");
        let chan_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std/chan.iz");

        let thread_src =
            fs::read_to_string(&thread_path).expect("failed to read std::thread fixture");
        let sync_src = fs::read_to_string(&sync_path).expect("failed to read std::sync fixture");
        let atomic_src =
            fs::read_to_string(&atomic_path).expect("failed to read std::atomic fixture");
        let async_src = fs::read_to_string(&async_path).expect("failed to read std::async fixture");
        let chan_src = fs::read_to_string(&chan_path).expect("failed to read std::chan fixture");

        let required_thread = [
            "shape JoinHandle<",
            "forge join(",
            "forge spawn<",
            "forge sleep",
            "forge park",
        ];
        let required_sync = [
            "shape Mutex<",
            "shape RwLock<",
            "shape Condvar",
            "shape Barrier",
            "shape Once",
        ];
        let required_atomic = ["scroll Ordering", "shape Atomic<", "forge fetch_add("];
        let required_chan = [
            "shape Sender<",
            "shape Receiver<",
            "forge send(",
            "flow forge recv(",
            "forge new<",
        ];
        let required_async = [
            "shape Flow<",
            "shape AsyncExecutor",
            "forge join<",
            "forge select<",
        ];

        for symbol in required_thread {
            assert!(
                thread_src.contains(symbol),
                "missing std::thread declaration: {}",
                symbol
            );
        }

        for symbol in required_sync {
            assert!(
                sync_src.contains(symbol),
                "missing std::sync declaration: {}",
                symbol
            );
        }

        for symbol in required_atomic {
            assert!(
                atomic_src.contains(symbol),
                "missing std::atomic declaration: {}",
                symbol
            );
        }

        for symbol in required_chan {
            assert!(
                chan_src.contains(symbol),
                "missing std::chan declaration: {}",
                symbol
            );
        }

        for symbol in required_async {
            assert!(
                async_src.contains(symbol),
                "missing std::async declaration: {}",
                symbol
            );
        }
    }

    #[test]
    fn std_atomic_exposes_surface() {
        let sync_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../library/std/atomic.iz");

        let src = fs::read_to_string(&sync_path).expect("failed to read std::atomic fixture");

        let required = [
            "scroll Ordering",
            "Relaxed",
            "Acquire",
            "Release",
            "AcqRel",
            "SeqCst",
            "shape Atomic<",
            "forge new(value: T) -> Atomic<T>",
            "forge load(&~self, _order: Ordering) -> T",
            "forge store(&~self, value: T, _order: Ordering)",
            "forge swap(&~self, value: T, _order: Ordering) -> T",
            "forge compare_exchange(",
            "forge fetch_add(&~self, val: T, _order: Ordering) -> T",
        ];

        for symbol in required {
            assert!(
                src.contains(symbol),
                "missing std::atomic declaration: {}",
                symbol
            );
        }
    }

    #[test]
    fn std_core_exposes_no_alloc_surface() {
        let checks: [(&str, &[&str]); 12] = [
            ("prim.iz", &["impl i32", "impl f64", "impl bool"]),
            (
                "ops.iz",
                &["weave Add", "weave Sub", "weave Mul", "weave Pipe"],
            ),
            ("cmp.iz", &["scroll Ordering", "weave Eq", "weave Ord"]),
            (
                "option.iz",
                &["scroll Option<", "forge is_some", "forge map<"],
            ),
            (
                "result.iz",
                &["scroll Result<", "shape Cascade<", "forge is_ok"],
            ),
            (
                "convert.iz",
                &[
                    "weave From<",
                    "weave Into<",
                    "weave TryFrom<",
                    "weave TryInto<",
                ],
            ),
            (
                "fmt.iz",
                &[
                    "shape Formatter",
                    "weave Display",
                    "weave Debug",
                    "macro format!",
                ],
            ),
            (
                "mem.iz",
                &[
                    "forge size_of<",
                    "forge align_of<",
                    "forge transmute<",
                    "forge drop<",
                ],
            ),
            (
                "ptr.iz",
                &[
                    "forge null<",
                    "forge null_mut<",
                    "forge write<",
                    "forge read<",
                ],
            ),
            ("slice.iz", &["shape Slice<", "forge len", "forge get"]),
            ("str.iz", &["impl str", "forge len", "forge starts_with"]),
            (
                "marker.iz",
                &[
                    "weave Copy",
                    "weave Send",
                    "weave Sync",
                    "weave Sized",
                    "weave Unpin",
                ],
            ),
        ];

        for (file_name, required) in checks {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../library/std/{}", file_name));
            let src = fs::read_to_string(&path).expect("failed to read std core fixture");

            for symbol in required {
                assert!(
                    src.contains(symbol),
                    "missing std::{} declaration: {}",
                    file_name,
                    symbol
                );
            }
        }
    }

    #[test]
    fn std_io_os_exposes_surface() {
        let checks: [(&str, &[&str]); 6] = [
            (
                "io.iz",
                &[
                    "forge println",
                    "forge eprintln",
                    "forge stdin",
                    "forge stdout",
                    "forge stderr",
                    "weave Read",
                    "weave Write",
                    "weave Seek",
                ],
            ),
            (
                "fs.iz",
                &[
                    "forge read_to_string",
                    "forge write",
                    "forge copy",
                    "forge create_dir",
                    "shape DirEntry",
                ],
            ),
            ("path.iz", &["shape Path", "shape PathBuf"]),
            ("env.iz", &["forge args", "forge vars", "forge current_dir"]),
            ("os.iz", &["OS-specific extensions"]),
            ("ffi.iz", &["shape CStr", "shape CString"]),
        ];

        for (file_name, required) in checks {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../library/std/{}", file_name));
            let src = fs::read_to_string(&path).expect("failed to read std io fixture");

            for symbol in required {
                assert!(
                    src.contains(symbol),
                    "missing std::{} declaration: {}",
                    file_name,
                    symbol
                );
            }
        }
    }

    #[test]
    fn std_math_hash_codec_exposes_surface() {
        let checks: [(&str, &[&str]); 5] = [
            (
                "math.iz",
                &[
                    "forge sin",
                    "forge cos",
                    "forge tan",
                    "forge exp",
                    "forge ln",
                    "forge log",
                    "PI",
                    "E",
                    "INFINITY",
                    "NAN",
                ],
            ),
            (
                "hash.iz",
                &[
                    "weave Hash",
                    "weave Hasher",
                    "shape DefaultHasher",
                    "forge finish",
                ],
            ),
            ("crypt.iz", &["BLAKE3", "SHA-2", "forge constant_time_eq"]),
            (
                "codec.iz",
                &[
                    "shape Base64",
                    "shape Hex",
                    "forge encode_base64",
                    "forge decode_base64",
                    "forge encode_hex",
                    "forge decode_hex",
                ],
            ),
            (
                "json.iz",
                &[
                    "JSON",
                    "round-trip",
                    "forge encode_json",
                    "forge decode_json",
                ],
            ),
        ];

        for (file_name, required) in checks {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../library/std/{}", file_name));
            let src = fs::read_to_string(&path).expect("failed to read std math fixture");

            for symbol in required {
                assert!(
                    src.contains(symbol),
                    "missing std::{} declaration: {}",
                    file_name,
                    symbol
                );
            }
        }
    }

    #[test]
    fn std_testing_exposes_surface() {
        let checks: [(&str, &[&str]); 3] = [
            (
                "test.iz",
                &["#[test]", "assert!", "assert_eq!", "should_panic!"],
            ),
            (
                "bench.iz",
                &["#[bench]", "shape Bencher", "forge black_box"],
            ),
            (
                "mock.iz",
                &[
                    "Mockable weave stubs for effect testing",
                    "weave MockEffect",
                ],
            ),
        ];

        for (file_name, required) in checks {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../../library/std/{}", file_name));
            let src = fs::read_to_string(&path).expect("failed to read std test fixture");

            for symbol in required {
                assert!(
                    src.contains(symbol),
                    "missing std::{} declaration: {}",
                    file_name,
                    symbol
                );
            }
        }
    }
}
