use core::str;
use rig::{completion::ToolDefinition, tool::Tool};
use serde_json::json;

use super::parser::ArxivParser;

const ARXIV_URL: &str = "http://export.arxiv.org/api/query";

#[derive(Debug, thiserror::Error)]
pub enum ArxivError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("XML parsing error: {0}")]
    XmlParsing(#[from] quick_xml::Error),
    #[error("No reqults found")]
    NoResults,
    #[error("UTF-8 decoding error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

// 定义保存论文元数据的结构体
#[derive(Debug, Default, serde::Deserialize, serde::Serialize, Clone)]
pub struct Paper {
    /// 标题
    pub title: String,
    /// 作者
    pub authors: Vec<String>,
    /// 摘要
    pub abstract_text: String,
    /// 地址
    pub url: String,
    /// 分类
    pub categories: Vec<String>,
}

#[derive(serde::Deserialize)]
pub struct SearchArgs {
    query: String,
    max_results: Option<i32>,
}

// 搜索论文的工具
#[derive(serde::Deserialize, serde::Serialize)]
pub struct ArxivSearchTool;

impl Tool for ArxivSearchTool {
    const NAME: &'static str = "search_arxiv";
    type Error = ArxivError;
    type Args = SearchArgs;
    type Output = Vec<Paper>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "search_arxiv".to_string(),
            description: "Search for academic papers on arXiv".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query for papers"
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of results to return (default: 5)"
                    }
                },
                "required": ["query", "max_results"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let max_results = args.max_results.unwrap_or(5);
        let client = reqwest::Client::new();

        let response = client
            .get(ARXIV_URL)
            .query(&[
                ("search_query", format!("all:{}", args.query)),
                ("start", 0.to_string()),
                ("max_results", max_results.to_string()),
            ])
            .send()
            .await?
            .text()
            .await?;

        Ok(ArxivParser::default().parse_response(&response)?)
    }
}

// HTML formatting function for papers
pub fn format_papers_as_html(papers: &[Paper]) -> Result<String, anyhow::Error> {
    let tpl = std::fs::read_to_string("static/table.html")?;
    let mut context = tera::Context::new();
    context.insert("papers", papers);

    let result = tera::Tera::one_off(&tpl, &context, false)?;

    Ok(result)
}
