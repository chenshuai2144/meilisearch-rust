use futures::executor::block_on;
use meilisearch_sdk::{client::*, document::*};
use pinyin::ToPinyin;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs::File;
use std::io::Read;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DocumentItem {
    url: String,
    title: String,
    content: String,
    tag: Option<String>,
    toc: Vec<DocumentItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct MeilisearchDocument {
    id: String,
    url: String,
    title: String,
    tag: String,
    content: String,
    pinyin: String,
}

// That trait is required to make a struct usable by an index
impl Document for MeilisearchDocument {
    type UIDType = String;

    fn get_uid(&self) -> &Self::UIDType {
        &self.id
    }
}

fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn to_pinyin(s: &String) -> String {
    let mut pinyin_str = String::new();
    for pinyin in s.as_str().to_pinyin() {
        if let Some(pinyin) = pinyin {
            pinyin_str = pinyin_str + " " + pinyin.plain();
        }
    }
    if pinyin_str.len() > 0 {
        pinyin_str.to_string()
    } else {
        s.clone()
    }
}

fn loop_insert(
    data: &DocumentItem,
    context: &mut Vec<MeilisearchDocument>,
    site_data: &DocumentItem,
    level: usize,
) {
    let site_name = site_data.title.split("-").collect::<Vec<&str>>()[0].trim();

    if data.toc.len() <= 0 {
        return;
    }
    let mut index = level;
    for item_data in data.toc.iter() {
        if item_data.title.len() > 0 {
            index = index + 1;

            context.push(MeilisearchDocument {
                id: index.to_string(),
                pinyin: to_pinyin(&data.content),
                url: item_data.url.clone() + " - " + site_name,
                title: item_data.title.clone(),
                content: item_data.content.clone(),
                tag: if item_data.tag.clone().is_none() {
                    "DOM".to_string()
                } else {
                    item_data.tag.clone().unwrap()
                },
            });
            if item_data.toc.len() > 0 {
                loop_insert(item_data, context, site_data, index * 10);
            }
        }
    }
}

fn main() {
    block_on(async move {
        // Create a client (without sending any request so that can't fail)
        let client = Client::new("http://localhost:7700", "masterKey");

        let mut file = File::open("./output.json").unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        // An index is where the documents are stored.
        let all_docs = client.index("docs");
        let data_list: Vec<DocumentItem> = serde_json::from_str(&data).unwrap();
        for data in data_list.iter() {
            all_docs
                .add_documents(
                    &[MeilisearchDocument {
                        id: "0".to_string(),
                        pinyin: to_pinyin(&data.content),
                        url: data.url.clone(),
                        title: data.title.clone(),
                        content: data.content.clone(),
                        tag: if data.tag.clone().is_none() {
                            "DOM".to_string()
                        } else {
                            data.tag.clone().unwrap()
                        },
                    }],
                    Some("id"),
                )
                .await
                .unwrap();

            let title = remove_whitespace(&mut data.title.clone());

            println!("正在插入：{:#?}", title);
            let docs = client.index(title);

            let mut documents: Vec<MeilisearchDocument> = Vec::new();

            loop_insert(&data, &mut documents, &data, 1);

            if documents.len() > 0 {
                match docs.add_documents(&documents, Some("id")).await {
                    Ok(_) => {}
                    Err(err) => {
                        print!("插入失败,{}", err.to_string());
                    }
                };
                match all_docs.add_documents(&documents, Some("id")).await {
                    Ok(_) => {}
                    Err(err) => {
                        print!("插入失败,{}", err.to_string());
                    }
                };
            }
        }
    });
}
