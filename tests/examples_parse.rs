//! Все примеры в папке examples/ должны компилироваться без ошибок —
//! регрессионный тест на синтаксис демо-сцен.

use std::path::Path;

#[test]
fn all_examples_parse() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("папка examples должна существовать") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("rx") {
            let src = std::fs::read_to_string(&path).unwrap();
            renderex::compile(&src)
                .unwrap_or_else(|d| panic!("{} не компилируется: {d}", path.display()));
            checked += 1;
        }
    }
    assert!(checked >= 4, "ожидались примеры сцен, найдено {checked}");
}
