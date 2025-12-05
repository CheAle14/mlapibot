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

            let original = version;

            $(
                if version < $idx {
                    println!("[db] applying {}::{} ({version})", stringify!($mod), stringify!($struct));

                    let migration = $mod::$struct;

                    match migration.apply(db) {
                        Ok(()) => {
                            version = $idx;
                            db.set_migration_version(version)?;
                        },
                        Err(err) => {
                            eprintln!("Failed to apply migration {} :: {}", $idx, stringify!($struct));
                            return Err(err);
                        }
                    }

                    if let Some(fixup) = migration.requires_manual_fixup() {
                        panic!("Halted migration as {} requires manual fixup: {fixup}", stringify!($struct));
                    }
                }
            )*

            if original != version {
                println!("[db] Migration complete!");
            }

            Ok(())
        }
    };
}

migrations!(
    1 => migration00::Initial,
    2 => migration01::ResolvedAt,
    3 => migration02::StickyStoreId,
    4 => migration03::StatusLiveThread,
    5 => migration04::StaffReplyComment,
    6 => migration05::StaffReplyStoreSubreddit,
    7 => migration06::StaffReplyHash,
    8 => migration07::StaffReplyPrefix
);

trait Migration {
    fn apply(&self, db: &mut MlapiDb) -> rusqlite::Result<()>;

    fn requires_manual_fixup(&self) -> Option<&'static str> {
        None
    }
}
