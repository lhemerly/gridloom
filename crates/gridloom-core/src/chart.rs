use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChartType {
    Line,
    Bar,
    Column,
    Scatter,
    Pie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartSeriesBinding {
    pub name: String,
    pub value_column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDefinition {
    pub id: String,
    pub title: String,
    pub chart_type: ChartType,
    pub source_table: String,
    pub category_column: String,
    pub series: Vec<ChartSeriesBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartDataSeries {
    pub name: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub title: String,
    pub chart_type: ChartType,
    pub categories: Vec<String>,
    pub series: Vec<ChartDataSeries>,
}
