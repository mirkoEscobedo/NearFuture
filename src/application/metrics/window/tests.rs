use super::Window;

#[test]
fn empty_window_and_zero_sample_have_distinct_quantiles() {
    let mut window = Window::new();
    let empty = window.summary(0).expect("registered empty policy window");
    assert_eq!((empty.completed, empty.retained, empty.evicted), (0, 0, 0));
    assert_eq!(empty.quantiles, [None; 4]);

    window.append(0, 0, 1).expect("actual scalar zero policy data");
    let zero = window.summary(1).expect("one retained zero policy sample");
    assert_eq!((zero.completed, zero.retained, zero.evicted), (1, 1, 0));
    assert_eq!(zero.quantiles, [Some(0); 4]);
}

#[test]
fn full_window_quantiles_and_oldest_eviction_match_independent_goldens() {
    let mut window = Window::new();
    for completed in 0_u64..64 {
        window
            .append(64 - completed, completed, completed + 1)
            .expect("bounded descending policy data");
    }
    let full = window.summary(64).expect("full retained policy window");
    assert_eq!((full.completed, full.retained, full.evicted), (64, 64, 0));
    assert_eq!(full.quantiles, [Some(32), Some(61), Some(64), Some(64)]);

    window.append(1000, 64, 65).expect("sixty-fifth policy sample");
    let evicted = window.summary(65).expect("oldest-evicted policy window");
    assert_eq!((evicted.completed, evicted.retained, evicted.evicted), (65, 64, 1));
    assert_eq!(evicted.quantiles, [Some(32), Some(61), Some(1000), Some(1000)]);
}
