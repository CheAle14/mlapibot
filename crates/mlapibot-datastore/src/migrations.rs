use crate::MlapiDb;

macro_rules! migrations {
    ($(
        $idx:literal => $mod:ident::$struct:ident
    ),* $(,)?) => {
        $(
            mod $mod;
        )*

        #[allow(unused_assignments)]
        pub fn ensure_updated(db: &mut MlapiDb) -> rusqlite::Result<()> {
            let mut version = db.get_migration_version()?;

            $(
                if version < $idx {
                    println!("[db] applying {}::{} ({version})", stringify!($mod), stringify!($struct));

                    match apply_migration(db, $idx, $mod::$struct::up) {
                        Ok(()) => version = $idx,
                        Err(err) => {
                            eprintln!("Failed to apply migration {} :: {}", $idx, stringify!($struct));
                            return Err(err);
                        }
                    }
                }
            )*

            Ok(())
        }
    };
}

migrations!(
    1 => migration00::Initial,
);

fn apply_migration(
    db: &mut MlapiDb,
    idx: u32,
    migration: impl FnOnce(&mut MlapiDb) -> rusqlite::Result<()>,
) -> rusqlite::Result<()> {
    migration(db)?;
    db.set_migration_version(idx)
}
