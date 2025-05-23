use std::convert::TryFrom;

use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;
use structured::{ColumnStatistics, Element};

use crate::{
    Error,
    schema::{description_binary::dsl as desc_d, top_n_binary::dsl as topn_d},
};

#[derive(Deserialize, Debug, Insertable, PartialEq)]
#[diesel(table_name = crate::schema::description_binary)]
struct DescriptionBinary<'a> {
    description_id: i32,
    mode: &'a [u8],
}

#[derive(Deserialize, Debug, Insertable, PartialEq)]
#[diesel(table_name = crate::schema::top_n_binary)]
struct TopNBinary<'a> {
    description_id: i32,
    value: Option<&'a [u8]>,
    count: i64,
}

pub(super) async fn insert_top_n(
    conn: &mut AsyncPgConnection,
    description_id: i32,
    column_stats: &ColumnStatistics,
    mode: &[u8],
) -> Result<usize, Error> {
    let db = DescriptionBinary {
        description_id,
        mode,
    };
    let _res = diesel::insert_into(desc_d::description_binary)
        .values(&db)
        .execute(conn)
        .await?;

    let top_n: Vec<_> = column_stats
        .n_largest_count
        .top_n()
        .iter()
        .map(|e| {
            let value = if let Element::Binary(binary) = &e.value {
                Some(binary.as_slice())
            } else {
                None
            };
            let count = i64::try_from(e.count).expect("Must be less than i64::MAX");
            TopNBinary {
                description_id,
                value,
                count,
            }
        })
        .collect();
    Ok(diesel::insert_into(topn_d::top_n_binary)
        .values(&top_n)
        .execute(conn)
        .await?)
}
