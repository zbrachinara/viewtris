#[test]
fn s_plus_game() {
    println!(
        "{:?}",
        serde_json::from_slice::<ttrm::Ttrm>(include_bytes!("HBSQabUhSS.ttrm")).unwrap()
    );
}
