//! > NOTE: This project is not affiliated with the Python [TinyDB](https://tinydb.readthedocs.io/en/latest/),
//! accidental naming error from when this project was started. See
//! [renaming](https://github.com/scOwez/tinydb/issues/3) for updates
//!
//! TinyDB or `tinydb` is a small-footprint, superfast database designed to be
//! used in-memory and easily dumped/retrieved from a file when it's time to save
//! ✨
//!
//! This database aims to provide an easy frontend to an efficiant in-memory
//! database (that can also be dumped to a file). It purposefully disallows
//! duplicate items to be sorted due to constraints with hash tables.
//!
//! ## Example 🚀
//!
//! A simple example of adding a structure then querying for it:
//!
//! ```rust
//! use serde::{Serialize, Deserialize};
//! use tinydb::Database;
//!
//! #[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Clone)]
//! struct ExampleStruct {
//!     my_age: i32
//! }
//!
//! fn main() {
//!     let my_struct = ExampleStruct { my_age: 329 };
//!     let mut my_db = Database::new("query_test", None, false);
//!
//!     my_db.add_item(my_struct.clone());
//!
//!     let results = my_db.query_item(|s: &ExampleStruct| &s.my_age, 329);
//!
//!     assert_eq!(results.unwrap(), &my_struct);
//! }
//! ```
//!
//! # Installation
//!
//! Simply add the following to your `Cargo.toml` file:
//!
//! ```toml
//! [dependencies]
//! tinydb = "1.0.0"
//! ```
//! # Implementation notes
//!
//! - This database does not save 2 duplicated items, either ignoring or raising an
//! error depending on end-user preference.
//! - This project is not intended to be used inside of any critical systems due to
//! the nature of dumping/recovery. If you are using this crate as a temporary and
//! in-memory only database, it should preform at a reasonable speed (as it uses
//! [HashSet] underneath).
//!
//! # Essential operations
//!
//! Some commonly-used operations for the [Database] structure.
//!
//! | Operation                               | Implamentation          |
//! |-----------------------------------------|-------------------------|
//! | Create database                         | [Database::new]         |
//! | Create database from file               | [Database::from]        |
//! | Load database or create if non-existant | [Database::auto_from]   |
//! | Query all matching items                | [Database::query]       |
//! | Query for item                          | [Database::query_item]  |
//! | Contains specific item                  | [Database::contains]    |
//! | Update/replace item                     | [Database::update_item] |
//! | Delete item                             | [Database::remove_item] |
//! | Dump database                           | [Database::dump_db]     |

#![doc(
    html_logo_url = "https://github.com/Owez/tinydb/raw/master/logo.png",
    html_favicon_url = "https://github.com/Owez/tinydb/raw/master/logo.png"
)]

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::hash;
use std::io::prelude::*;
use std::path::PathBuf;

pub mod error;

/// The primary database structure, allowing storage of a generic type with
/// dumping/saving options avalible.
///
/// The generic type used should primarily be structures as they resemble a
/// conventional database model and should implament [hash::Hash] and [Eq] for
/// basic in-memory storage with [Serialize] and [Deserialize] being implamented
/// for file operations involving the database (these are also required).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Database<T: hash::Hash + Eq> {
    /// Friendly name for the database, preferibly in `slug-form-like-this` as
    /// this is the fallback path
    ///
    /// This is used when dumping the database without a [Database::save_path]
    /// being defined and a friendly way to order a database
    pub label: String,

    /// The overwrite path to save the database as, this is recommended otherwise
    /// it will end up as `./Hello\ There.tinydb` if [Database::label] is "Hello
    /// There"
    ///
    /// Primarily used inside of [Database::dump_db].
    pub save_path: Option<PathBuf>,

    /// If the database should return an error if it tries to insert where an
    /// identical item already is. Setting this as `false` doesn't allow
    /// duplicates, it just doesn't flag an error.
    pub strict_dupes: bool,

    /// In-memory [HashSet] of all items
    pub items: HashSet<T>,
}

impl<T: hash::Hash + Eq + Serialize + DeserializeOwned> Database<T> {
    /// Creates a new database instance from given parameters.
    ///
    /// - To add a first item, use [Database::add_item].
    /// - If you'd like to load a dumped database, use [Database::from].
    pub fn new(
        label: impl Into<String>,
        save_path: impl Into<Option<PathBuf>>,
        strict_dupes: bool,
    ) -> Self {
        Database {
            label: label.into(),
            save_path: save_path.into(),
            strict_dupes,
            items: HashSet::new(),
        }
    }

    /// Creates a database from a `.tinydb` file.
    ///
    /// This retrives a dump file (saved database) from the path given and loads
    /// it as the [Database] structure.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tinydb::Database;
    /// use serde::{Serialize, Deserialize};
    /// use std::path::PathBuf;
    ///
    /// /// Small example structure to show.
    /// #[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
    /// struct ExampleStruct {
    ///    data: i32
    /// }
    ///
    /// /// Makes a small testing database.
    /// fn make_db() {
    ///     let mut test_db = Database::new("test", None, false);
    ///     test_db.add_item(ExampleStruct { data: 34 });
    ///     test_db.dump_db();
    /// }
    ///
    /// /// Get `test_db` defined in [make_db] and test.
    /// fn main() {
    ///     make_db();
    ///
    ///     let got_db = Database::from(
    ///         PathBuf::from("test.tinydb")
    ///     ).unwrap();
    ///
    ///     assert_eq!(
    ///         got_db.query_item(|s: &ExampleStruct| &s.data, 34).unwrap(),
    ///         &ExampleStruct { data: 34 }
    ///     ); // Check that the database still has added [ExampleStruct].
    /// }
    /// ```
    pub fn from(path: impl Into<PathBuf>) -> Result<Self, error::DatabaseError> {
        let stream = get_stream_from_path(path.into())?;
        let decoded: Database<T> = bincode::deserialize(&stream[..]).unwrap();

        Ok(decoded)
    }

    /// Loads database from existant path or creates a new one if it doesn't already
    /// exist.
    ///
    /// This is the recommended way to use TinyDB if you are wanting to easily
    /// setup an entire database instance in a short, consise manner. Similar to
    /// [Database::new] and [Database::from], this function will also have to be
    /// given a strict type argument and you will still have to provide `script_dupes`
    /// even if the database is likely to load an existing one.
    ///
    /// This function does make some assumptions about the database name and uses
    /// the 2nd to last part before a `.`. This means that `x.y.z` will have the
    /// name of `y`, not `x` so therefore it is recommended to have a database
    /// path with `x.tinydb` or `x.db` only.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tinydb::*;
    /// use std::path::PathBuf;
    /// use serde::{Serialize, Deserialize};
    ///
    /// /// Small example structure to show.
    /// #[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
    /// struct ExampleStruct {
    ///    data: i32
    /// }
    ///
    /// fn main() {
    ///     let dummy_db: Database<ExampleStruct> = Database::new("cool", None, false); // create demo db for `db_from`
    ///
    ///     let db_from_path = PathBuf::from("cool.tinydb");
    ///     let db_from: Database<ExampleStruct> = Database::auto_from(db_from_path, false).unwrap(); // automatically load it
    ///
    ///     let db_new_path = PathBuf::from("xyz.tinydb");
    ///     let db_new: Database<ExampleStruct> = Database::auto_from(db_new_path, false).unwrap(); // automatically create new as "xyz" doesn't exist
    /// }
    /// ```
    pub fn auto_from(
        path: impl Into<PathBuf>,
        strict_dupes: bool,
    ) -> Result<Self, error::DatabaseError> {
        let path_into = path.into();

        if path_into.exists() {
            Database::from(path_into)
        } else {
            let db_name = match path_into.file_stem() {
                Some(x) => match x.to_str() {
                    Some(y) => String::from(y),
                    None => return Err(error::DatabaseError::BadDbName),
                },
                None => return Err(error::DatabaseError::BadDbName),
            };

            Ok(Database::new(db_name, Some(path_into), strict_dupes))
        }
    }

    /// Adds a new item to the in-memory database.
    ///
    /// If this is the first item added to the database, please ensure it's the
    /// only type you'd like to add. Due to generics, the first item you add
    /// will be set as the type to use (unless removed).
    pub fn add_item(&mut self, item: T) -> Result<(), error::DatabaseError> {
        if self.strict_dupes {
            if self.items.contains(&item) {
                return Err(error::DatabaseError::DupeFound);
            }
        }

        self.items.insert(item);
        return Ok(());
    }

    /// Replaces an item inside of the database with another
    /// item, used for updating/replacing items easily.
    ///
    /// [Database::query_item] can be used in conjunction to find and replace
    /// values individually if needed.
    pub fn update_item(&mut self, item: &T, new: T) -> Result<(), error::DatabaseError> {
        self.remove_item(item)?;
        self.add_item(new)?;

        Ok(())
    }

    /// Removes an item from the database.
    ///
    /// See [Database::update_item] if you'd like to update/replace an item easily,
    /// rather than individually deleting and adding.
    ///
    /// # Errors
    ///
    /// Will return [error::DatabaseError::ItemNotFound] if the item that is attempting
    /// to be deleted was not found.
    pub fn remove_item(&mut self, item: &T) -> Result<(), error::DatabaseError> {
        if self.items.remove(item) {
            Ok(())
        } else {
            Err(error::DatabaseError::ItemNotFound)
        }
    }

    /// Dumps/saves database to a binary file.
    ///
    /// # Saving path methods
    ///
    /// The database will usually save as `\[label\].tinydb` where `\[label\]`
    /// is the defined [Database::label] (path is reletive to where tinydb was
    /// executed).
    ///
    /// You can also overwrite this behaviour by defining a [Database::save_path]
    /// when generating the database inside of [Database::new].
    pub fn dump_db(&self) -> Result<(), error::DatabaseError> {
        let mut dump_file = self.open_db_path()?;
        bincode::serialize_into(&mut dump_file, self).unwrap();

        Ok(())
    }

    /// Query the database for a specific item.
    ///
    /// # Syntax
    ///
    /// ```none
    /// self.query_item(|[p]| [p].[field], [query]);
    /// ```
    ///
    /// - `[p]` The closure (Will be whatever the database currently is saving as a schema).
    /// - `[field]` The exact field of `p`. If the database doesn't contain structures, don't add the `.[field]`.
    /// - `[query]` Item to query for. This is a generic and can be of any reasonable type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use serde::{Serialize, Deserialize};
    /// use tinydb::Database;
    ///
    /// #[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Clone)]
    /// struct ExampleStruct {
    ///     my_age: i32
    /// }
    ///
    /// fn main() {
    ///     let my_struct = ExampleStruct { my_age: 329 };
    ///     let mut my_db = Database::new("query_test", None, false);
    ///
    ///     my_db.add_item(my_struct.clone());
    ///
    ///     let results = my_db.query_item(|s: &ExampleStruct| &s.my_age, 329);
    ///
    ///     assert_eq!(results.unwrap(), &my_struct);
    /// }
    /// ```
    pub fn query_item<Q: PartialEq, V: Fn(&T) -> &Q>(
        &self,
        value: V,
        query: Q,
    ) -> Result<&T, error::DatabaseError> {
        for item in self.items.iter() {
            if value(item) == &query {
                return Ok(item);
            }
        }

        Err(error::DatabaseError::ItemNotFound)
    }

    /// Query the database for all matching items.
    ///
    /// # Syntax
    ///
    /// ```none
    /// self.query(|[p]| [p].[field], [query]);
    /// ```
    ///
    /// - `[p]` The closure (Will be whatever the database currently is saving as a schema).
    /// - `[field]` The exact field of `p`. If the database doesn't contain structures, don't add the `.[field]`.
    /// - `[query]` Item to query for. This is a generic and can be of any reasonable type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use serde::{Serialize, Deserialize};
    /// use tinydb::Database;
    ///
    /// #[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Clone)]
    /// struct ExampleStruct {
    ///     uuid: String,
    ///     age: i32,
    /// }
    ///
    /// fn main() {
    ///     let mut my_db = Database::new("query_test", None, false);
    ///
    ///     my_db.add_item(ExampleStruct { uuid: "test1".into(), age: 20 });
    ///     my_db.add_item(ExampleStruct { uuid: "test2".into(), age: 20 });
    ///     my_db.add_item(ExampleStruct { uuid: "test3".into(), age: 18 });
    ///
    ///     let results = my_db.query(|s: &ExampleStruct| &s.age, 20);
    ///
    ///     assert_eq!(results.unwrap().len(), 2);
    /// }
    /// ```
    pub fn query<Q: PartialEq, V: Fn(&T) -> &Q>(
        &self,
        value: V,
        query: Q,
    ) -> Result<Vec<&T>, error::DatabaseError> {
        let mut items: Vec<&T> = vec![];
        for item in self.items.iter() {
            if value(item) == &query {
                items.push(item);
            }
        }

        if items.len() > 0 {
            return Ok(items);
        }

        Err(error::DatabaseError::ItemNotFound)
    }

    /// Searches the database for a specific value. If it does not exist, this
    /// method will return [error::DatabaseError::ItemNotFound].
    ///
    /// This is a wrapper around [HashSet::contains].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tinydb::Database;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Copy, Clone)]
    /// struct ExampleStruct {
    ///     item: i32
    /// }
    ///
    /// fn main() {
    ///     let exp_struct = ExampleStruct { item: 4942 };
    ///     let mut db = Database::new("Contains example", None, false);
    ///
    ///     db.add_item(exp_struct.clone());
    ///
    ///     assert_eq!(db.contains(&exp_struct), true);
    /// }
    /// ```
    pub fn contains(&self, query: &T) -> bool {
        self.items.contains(query)
    }

    /// Returns the number of database entries
    /// method will return i32.
    ///
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tinydb::Database;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Hash, Eq, PartialEq, Serialize, Deserialize, Copy, Clone)]
    /// struct ExampleStruct {
    ///     item: i32
    /// }
    ///
    /// fn main() {
    ///     let exp_struct = ExampleStruct { item: 4942 };
    ///     let mut db = Database::new("Contains example", None, false);
    ///
    ///     db.add_item(exp_struct.clone());
    ///
    ///     assert_eq!(db.len(), 1);
    /// }
    /// ```
    pub fn len(&self) -> i32 {
        self.items.len() as i32
    }

    /// Opens the path given in [Database::save_path] (or auto-generates a path).
    fn open_db_path(&self) -> Result<File, error::DatabaseError> {
        let definate_path = self.smart_path_get();

        if definate_path.exists() {
            std::fs::remove_file(&definate_path)?;
        }

        Ok(File::create(&definate_path)?)
    }

    /// Automatically allocates a path for the database if [Database::save_path]
    /// is not provided. If it is, this function will simply return it.
    fn smart_path_get(&self) -> PathBuf {
        if self.save_path.is_none() {
            return PathBuf::from(format!("{}.tinydb", self.label));
        }

        PathBuf::from(self.save_path.as_ref().unwrap())
    }
}

/// Reads a given path and converts it into a [Vec]<[u8]> stream.
fn get_stream_from_path(path: PathBuf) -> Result<Vec<u8>, error::DatabaseError> {
    if !path.exists() {
        return Err(error::DatabaseError::DatabaseNotFound);
    }

    let mut file = File::open(path)?;
    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer)?;

    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dummy struct to use inside of tests
    #[derive(Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
    struct DemoStruct {
        name: String,
        age: i32,
    }

    /// Tests addition to in-memory db
    #[test]
    fn item_add() -> Result<(), error::DatabaseError> {
        let mut my_db = Database::new("Adding test", None, true);

        my_db.add_item(DemoStruct {
            name: String::from("John"),
            age: 16,
        })?;

        Ok(())
    }

    /// Tests removal from in-memory db
    #[test]
    fn item_remove() -> Result<(), error::DatabaseError> {
        let mut my_db = Database::new("Removal test", None, true);

        let testing_struct = DemoStruct {
            name: String::from("Xander"),
            age: 33,
        };

        my_db.add_item(testing_struct.clone())?;
        my_db.remove_item(&testing_struct)?;

        Ok(())
    }

    #[test]
    fn db_dump() -> Result<(), error::DatabaseError> {
        let mut my_db = Database::new(
            String::from("Dumping test"),
            Some(PathBuf::from("test.tinydb")),
            true,
        );

        my_db.add_item(DemoStruct {
            name: String::from("Xander"),
            age: 33,
        })?;
        my_db.add_item(DemoStruct {
            name: String::from("John"),
            age: 54,
        })?;

        my_db.dump_db()?;

        Ok(())
    }
    /// Tests [Database::query_item]
    #[test]
    fn query_item_db() {
        let mut my_db = Database::new(
            String::from("Query test"),
            Some(PathBuf::from("test.tinydb")),
            true,
        );

        my_db
            .add_item(DemoStruct {
                name: String::from("Rimmer"),
                age: 5,
            })
            .unwrap();
        my_db
            .add_item(DemoStruct {
                name: String::from("Cat"),
                age: 10,
            })
            .unwrap();
        my_db
            .add_item(DemoStruct {
                name: String::from("Kryten"),
                age: 3000,
            })
            .unwrap();
        my_db
            .add_item(DemoStruct {
                name: String::from("Lister"),
                age: 62,
            })
            .unwrap();

        assert_eq!(
            my_db.query_item(|f| &f.age, 62).unwrap(),
            &DemoStruct {
                name: String::from("Lister"),
                age: 62,
            }
        ); // Finds "Lister" by searching [DemoStruct::age]
        assert_eq!(
            my_db.query_item(|f| &f.name, String::from("Cat")).unwrap(),
            &DemoStruct {
                name: String::from("Cat"),
                age: 10,
            }
        ); // Finds "Cat" by searching [DemoStruct::name]
    }

    /// Tests [Database::query]
    #[test]
    fn query_db() {
        let mut my_db = Database::new(
            String::from("Query test"),
            Some(PathBuf::from("test.tinydb")),
            false,
        );

        my_db
            .add_item(DemoStruct {
                name: String::from("Rimmer"),
                age: 5,
            })
            .unwrap();
        my_db
            .add_item(DemoStruct {
                name: String::from("Cat"),
                age: 10,
            })
            .unwrap();
        my_db
            .add_item(DemoStruct {
                name: String::from("Kryten"),
                age: 3000,
            })
            .unwrap();
        my_db
            .add_item(DemoStruct {
                name: String::from("Lister"),
                age: 62,
            })
            .unwrap();

        my_db
            .add_item(DemoStruct {
                name: String::from("Lister"),
                age: 64,
            })
            .unwrap();

        assert_eq!(
            my_db
                .query(|f| &f.name, String::from("Lister"))
                .unwrap()
                .len(),
            2
        ); // Finds "Lister" by searching [DemoStruct::name]
    }

    /// Tests a [Database::from] method call
    #[test]
    fn db_from() -> Result<(), error::DatabaseError> {
        let mut my_db = Database::new(
            String::from("Dumping test"),
            Some(PathBuf::from("test.tinydb")),
            false,
        );

        let demo_mock = DemoStruct {
            name: String::from("Xander"),
            age: 33,
        };

        my_db.add_item(demo_mock.clone()).unwrap();

        my_db.dump_db()?;

        let db: Database<DemoStruct> = Database::from(PathBuf::from("test.tinydb"))?;
        assert_eq!(db.label, String::from("Dumping test"));

        Ok(())
    }

    /// Test if the database contains that exact item, related to
    /// [Database::contains].
    #[test]
    fn db_contains() {
        let exp_struct = DemoStruct {
            name: String::from("Xander"),
            age: 33,
        };

        let mut db = Database::new(String::from("Contains example"), None, false);
        db.add_item(exp_struct.clone()).unwrap();
        assert_eq!(db.contains(&exp_struct), true);
    }

    /// Tests [Database::auto_from]'s ability to create new databases and fetch
    /// already existing ones; an all-round test of its purpose.
    #[test]
    fn auto_from_creation() {
        let _dummy_db: Database<DemoStruct> =
            Database::new(String::from("alreadyexists"), None, false);

        let from_db_path = PathBuf::from("alreadyexists.tinydb");
        let _from_db: Database<DemoStruct> = Database::auto_from(from_db_path, false).unwrap();

        let new_db_path = PathBuf::from("nonexistant.tinydb");
        let _net_db: Database<DemoStruct> = Database::auto_from(new_db_path, false).unwrap();
    }

    /// Tests [Database::len] returns the number of database entries
    #[test]
    fn len() {
        let mut db: Database<DemoStruct> = Database::new(
            String::from("Query test"),
            Some(PathBuf::from("test.tinydb")),
            true,
        );

        let demo_mock = DemoStruct {
            name: String::from("Xander"),
            age: 33,
        };

        db.add_item(demo_mock.clone()).unwrap();

        assert_eq!(db.len(), 1);
    }
}
