use std::path::{Path, PathBuf};
use structopt::StructOpt;

use cursive::{
    event::Key,
    logger::{self, Record},
    traits::Nameable,
    views::{self, Dialog, PaddedView, SelectView, StackView},
    Cursive,
};
use serde::Deserialize;

const TOKEN: &'static str = "WAgjEI7Y1bnIHREU72CdySxOzeFymC0L";

#[derive(Debug, Deserialize)]
struct AssrtListResponse {
    status: u32,
    sub: ListObject,
}

#[derive(Debug, Deserialize)]
struct AssrtDetailResponse {
    status: u32,
    sub: DetailObject,
}

#[derive(Debug, Deserialize)]
struct DetailObject {
    subs: Vec<DetailSubObject>,
}

#[derive(Debug, Deserialize)]
struct DetailSubObject {
    id: u32,
    down_count: u32,
    upload_time: String,
    filename: String,
    url: String,
    filelist: SubFileList,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SubFileList {
    Empty {},
    List(Vec<SubFile>),
}

#[derive(Debug, Deserialize)]
struct SubFile {
    url: String,
    f: String,
    s: String,
}

#[derive(Debug, Deserialize)]
struct ListObject {
    subs: Vec<Sub>,
    action: String,
    result: String,
    keyword: String,
}

#[derive(Debug, Deserialize)]
struct Sub {
    native_name: String,
    videoname: String,
    revision: u32,
    subtype: String,
    upload_time: String,
    vote_score: u32,
    id: u32,
    #[serde(default)]
    release_site: String,
    #[serde(default)]
    lang: Lang,
}

#[derive(Debug, Default, Deserialize)]
struct Lang {
    desc: String,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "shoot", about = "Fetch subs from shooter fake")]
struct Opt {
    #[structopt(parse(from_os_str))]
    dir: PathBuf,
}

fn run(dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let keyword = dir.file_name().and_then(|s| s.to_str()).unwrap();
    let api_search = format!(
        "https://api.assrt.net/v1/sub/search?token={}&q={}",
        TOKEN, keyword
    );
    println!("Searching {}", api_search);
    let res: AssrtListResponse = reqwest::blocking::get(api_search)?.json()?;
    // https://secure.assrt.net/api/doc#%E9%94%99%E8%AF%AF%E5%80%BC
    match res.status {
        0 => {
            let mut siv = cursive::default();
            let mut stack_view = StackView::new().with_name("stack");
            let mut list_view = SelectView::new();
            let subs = res.sub.subs;

            for sub in subs {
                list_view.add_item(
                    format!(
                        "{}, score: {}, source: {}, lang: {}",
                        sub.videoname, sub.vote_score, sub.release_site, sub.lang.desc,
                    ),
                    sub.id,
                );
            }
            list_view.set_on_submit(|s, item| {
                let id = item;
                let api_detail = format!(
                    "https://api.assrt.net/v1/sub/detail?token={}&id={}",
                    TOKEN, id
                );
                let res: Result<AssrtDetailResponse, _> =
                    reqwest::blocking::get(&api_detail).and_then(|r| r.json());
                match res {
                    Err(e) => {
                        s.add_layer(Dialog::info(format!("Get {} failed {:?}", api_detail, e)))
                    }
                    Ok(res) => {
                        let mut detail_view = SelectView::new();
                        for item in res.sub.subs {
                            match item.filelist {
                                SubFileList::List(filelist) => {
                                    for file in filelist {
                                        detail_view.add_item(file.f.clone(), (file.f, file.url));
                                    }
                                }
                                SubFileList::Empty {} => {
                                    detail_view
                                        .add_item(item.filename.clone(), (item.filename, item.url));
                                }
                            }
                        }
                        detail_view.set_on_submit(|s, item| {
                            // s.add_layer(Dialog::info(format!("detail on submit, item {:?}", item)));
                            // s.add_layer(Dialog::info(format!("get detail, dir {:?}", dir)));
                            let (filename, url) = item;
                            let res = reqwest::blocking::get(url).and_then(|r| r.text());
                            s.add_layer(match res {
                                Err(e) => Dialog::info(format!("Download {} failed {:?}", url, e)),
                                Ok(text) => {
                                    let path = filename;
                                    match std::fs::write(path, text) {
                                        Err(e) => Dialog::info(format!(
                                            "Write to {:?} failed {:?}",
                                            path, e
                                        )),
                                        Ok(_) => Dialog::info(format!(
                                            "Save {} to {:?} successfully",
                                            url, path
                                        )),
                                    }
                                }
                            });
                        });
                        s.call_on_name("stack", |view: &mut views::StackView| {
                            view.add_layer(PaddedView::lrtb(2, 2, 1, 1, detail_view));
                        });
                    }
                }
            });
            stack_view.get_mut().add_layer(list_view);
            siv.add_layer(stack_view);
            siv.add_global_callback('q', |s| s.quit());
            siv.add_global_callback(Key::Backspace, |s| {
                s.call_on_name("stack", |view: &mut views::StackView| {
                    if view.len() > 1 {
                        view.pop_layer();
                    }
                });
            });
            siv.run();
        }
        1 => println!("no such user"),
        101 => println!("length of keyword must be longer than 3"),
        20000 => println!("your request is missing essential arguments"),
        20001 => println!("invalid token"),
        20400 => println!("API endpoint not found"),
        20900 => println!("subtitle not found"),
        30000 => println!("server is encounting errors"),
        30001 => println!("database is unavailable"),
        30002 => println!("search engine is unavailable"),
        30300 => println!("API is temporarily unavailable"),
        30900 => println!("you are exceeding request limits"),
        _ => println!("Unknown status {}", res.status),
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    let dir = opt.dir;
    if dir.is_dir() {
        run(dir)
    } else {
        println!("Invalid directory {:?}", dir);
        Ok(())
    }
}
