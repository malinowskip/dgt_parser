# DGT-Translation Memory Parser

A command-line tool for converting European Union’s [DGT-Translation Memory](https://joint-research-centre.ec.europa.eu/language-technology-resources/dgt-translation-memory_en) – a multilingual corpus of EU’s legislative documents – from TMX to other output formats (currently only a SQLite database).

The translation memory is distributed as a collection of [ZIP files](https://joint-research-centre.ec.europa.eu/language-technology-resources/dgt-translation-memory_en#download), each containing a set TMX (*Translation Memory eXchange*) files, each corresponding to a EUR-Lex document. Translation units contain parallel texts in up to 24 languages.

## Installation

With the [Rust toolchain installed](https://doc.rust-lang.org/cargo/getting-started/installation.html), build the binary from source:

```shell
git clone git@github.com:malinowskip/dgt_parser.git
cd dgt_parser
cargo build --release
```

The generated binary will be located at the following path: `./target/release/dgt_parser`.

## Basic Usage

```shell
dgt_parser -i <INPUT_DIR> <SUBCOMMAND>
```
...where the input dir is the directory containing the downloaded ZIP files and the subcommand specifies the output format (more details below).

## SQLite Database
The following command will save all translation units in an SQLite database.

```shell
dgt_parser -i <INPUT_DIR> sqlite --output db.sqlite
```

To display the schema of the generated database, you can run the following command (assuming SQLite is installed on your system):

```shell
sqlite3 my_output.db ".schema"
```

The database will contain two tables: `translation_units` and `documents`. The latter is a list of source EU documents. Each translation unit belongs to a document, and the `translation_units` table uses the `document_id` column as the foreign key referencing the corresponding document id.

For convenience, each translation unit is assigned a `sequential_number`, which is its consecutive number in the document it belongs to.

### Examples
Using the generated SQLite database:

#### Basic querying

```sql
--- EXAMPLE (joining translation units with documents) ---
SELECT
    tu.en_gb,
    d.name
FROM translation_units tu
JOIN documents d on tu.document_id = d.id
LIMIT 5;
```

#### Generating a full-text search index
*Note: the [FTS5 extension](https://www.sqlite.org/fts5.html) is required for this.*

```sql
CREATE VIRTUAL TABLE translation_units_fts USING fts5 (
    document_id,
    en_gb,
    pl_01,
    content=translation_units
);

INSERT INTO
    translation_units_fts
SELECT
    document_id,
    en_gb,
    pl_01
FROM
    translation_units;

--- Run full-text search queries ---
SELECT * FROM translation_units_fts WHERE en_gb MATCH 'tamper evident' LIMIT 5;

--- Include the name/id of the source document ---
SELECT
    d.name,
    tu.en_gb,
    tu.pl_01
FROM 
    translation_units_fts tu
JOIN documents d on d.id = tu.document_id
WHERE en_gb MATCH 'tamper evident'
LIMIT 5;
```

## Reference

Parse ZIP files in `./input_dir` and save all translation units in a SQLite database:
```shell
dgt_parser -i ./input_dir sqlite -o db.sqlite
```
---

Same as above, but only save phrases in Polish and in English, ignoring other languages. Additional language codes can be included by repeating the `-l <LANG_CODE>` option.
```shell
dgt_parser -l pl -l en -i ./input_dir sqlite -o db.sqlite
```

---

Same as above, but only include the translation units that contain texts in all of the specified languages.

```shell
dgt_parser --require-each-lang -l pl -l en -i ./input_dir sqlite -o db.sqlite
```