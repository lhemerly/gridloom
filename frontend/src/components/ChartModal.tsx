import React, { useState, useEffect } from 'react';
import { X, BarChart2, Play } from 'lucide-react';
import { api } from '../api';
import { ChartType, ChartDefinition, ChartData, WorksheetTable } from '../types';

interface ChartModalProps {
  isOpen: boolean;
  onClose: () => void;
  tables: WorksheetTable[];
}

const COLORS = [
  '#3b82f6', // blue
  '#10b981', // emerald
  '#f59e0b', // amber
  '#ef4444', // red
  '#8b5cf6', // purple
  '#ec4899', // pink
  '#06b6d4', // cyan
];

export const ChartModal: React.FC<ChartModalProps> = ({ isOpen, onClose, tables }) => {
  const [chartTitle, setChartTitle] = useState('Sales by Region');
  const [chartType, setChartType] = useState<ChartType>('Column');
  const [sourceTable, setSourceTable] = useState('');
  const [availableColumns, setAvailableColumns] = useState<string[]>([]);
  const [categoryCol, setCategoryCol] = useState('');
  const [valueCol, setValueCol] = useState('');

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [chartData, setChartData] = useState<ChartData | null>(null);

  useEffect(() => {
    if (tables.length > 0 && !sourceTable) {
      setSourceTable(tables[0].name);
      setAvailableColumns(tables[0].columns);
      if (tables[0].columns.length >= 2) {
        setCategoryCol(tables[0].columns[0]);
        setValueCol(tables[0].columns[1]);
      }
    }
  }, [tables, sourceTable]);

  const handleTableChange = (tName: string) => {
    setSourceTable(tName);
    const tbl = tables.find((t) => t.name === tName);
    if (tbl) {
      setAvailableColumns(tbl.columns);
      if (tbl.columns.length >= 2) {
        setCategoryCol(tbl.columns[0]);
        setValueCol(tbl.columns[1]);
      }
    }
  };

  const handleGenerate = async () => {
    if (!sourceTable || !categoryCol || !valueCol) return;
    setLoading(true);
    setError(null);
    try {
      const def: ChartDefinition = {
        id: `chart_${Date.now()}`,
        title: chartTitle,
        chart_type: chartType,
        source_table: tables.find(t => t.name === sourceTable)?.source_name ?? sourceTable,
        category_column: categoryCol,
        series: [{ name: valueCol, value_column: valueCol }],
      };
      await api.saveChart(def);
      const data = await api.getChartData(def.id);
      setChartData(data);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4">
      <div className="bg-white rounded-lg shadow-xl w-full max-w-4xl h-[580px] flex flex-col overflow-hidden border border-slate-200">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-slate-200 bg-slate-50">
          <div className="flex items-center gap-2">
            <BarChart2 className="w-5 h-5 text-blue-600" />
            <h2 className="text-sm font-bold text-slate-800">Chart Visualization Studio</h2>
          </div>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-slate-600 p-1 rounded hover:bg-slate-200"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {error && (
          <div className="bg-rose-50 text-rose-800 text-xs px-4 py-2 border-b border-rose-200 flex justify-between items-center">
            <span>{error}</span>
            <button onClick={() => setError(null)} className="font-bold underline text-[10px]">
              Dismiss
            </button>
          </div>
        )}

        {/* Content Body */}
        <div className="flex-1 flex overflow-hidden">
          {/* Left Config Panel */}
          <div className="w-80 border-r border-slate-200 p-4 bg-slate-50/50 flex flex-col gap-3 text-xs">
            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Chart Title</label>
              <input
                type="text"
                value={chartTitle}
                onChange={(e) => setChartTitle(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs"
              />
            </div>

            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Chart Type</label>
              <select
                value={chartType}
                onChange={(e) => setChartType(e.target.value as ChartType)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs font-semibold"
              >
                <option value="Column">Column Chart</option>
                <option value="Bar">Bar Chart</option>
                <option value="Line">Line Chart</option>
                <option value="Pie">Pie Chart</option>
                <option value="Scatter">Scatter Plot</option>
              </select>
            </div>

            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">Source Table</label>
              <select
                value={sourceTable}
                onChange={(e) => handleTableChange(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs font-medium"
              >
                {tables.map((t) => (
                  <option key={t.name} value={t.name}>
                    {t.name}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">
                Category / Dimension (X-Axis)
              </label>
              <select
                value={categoryCol}
                onChange={(e) => setCategoryCol(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs"
              >
                {availableColumns.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </div>

            <div>
              <label className="block text-[11px] font-bold text-slate-700 mb-1">
                Value Column (Y-Axis)
              </label>
              <select
                value={valueCol}
                onChange={(e) => setValueCol(e.target.value)}
                className="w-full px-2.5 py-1.5 border border-slate-300 rounded bg-white text-xs"
              >
                {availableColumns.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </div>

            <div className="pt-2 mt-auto">
              <button
                onClick={handleGenerate}
                disabled={loading || !sourceTable || !categoryCol || !valueCol}
                className="w-full flex items-center justify-center gap-1.5 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-bold shadow-sm transition-colors disabled:opacity-50"
              >
                <Play className="w-3.5 h-3.5 fill-white" />
                <span>{loading ? 'Rendering...' : 'Render Chart'}</span>
              </button>
            </div>
          </div>

          {/* Right SVG Chart Viewer */}
          <div className="flex-1 p-6 bg-white overflow-hidden flex flex-col items-center justify-center">
            {chartData && chartData.series.length > 0 ? (
              <div className="w-full h-full flex flex-col">
                <div className="text-center font-bold text-slate-800 text-sm mb-4">
                  {chartData.title}
                </div>
                <div className="flex-1 relative flex items-center justify-center">
                  <SvgChartViewer chartData={chartData} />
                </div>
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center text-slate-400 text-xs">
                <BarChart2 className="w-10 h-10 text-slate-300 mb-2" />
                <span>Select a table and bindings, then click Render Chart</span>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

// Interactive SVG Chart Renderer
const SvgChartViewer: React.FC<{ chartData: ChartData }> = ({ chartData }) => {
  const { chart_type, categories, series } = chartData;
  const values = series[0]?.values || [];
  const maxVal = Math.max(...values, 1);
  const minVal = Math.min(0, ...values);
  const range = maxVal - minVal || 1;

  const width = 500;
  const height = 300;
  const padding = 40;

  if (chart_type === 'Pie') {
    const total = values.reduce((sum, v) => sum + Math.max(0, v), 0) || 1;
    let accumulatedAngle = 0;
    const cx = width / 2;
    const cy = height / 2;
    const r = Math.min(width, height) / 2 - 30;

    return (
      <svg width={width} height={height} className="overflow-visible">
        {values.map((v, i) => {
          const sliceAngle = (Math.max(0, v) / total) * 2 * Math.PI;
          const startAngle = accumulatedAngle;
          accumulatedAngle += sliceAngle;
          const endAngle = accumulatedAngle;

          const x1 = cx + r * Math.cos(startAngle);
          const y1 = cy + r * Math.sin(startAngle);
          const x2 = cx + r * Math.cos(endAngle);
          const y2 = cy + r * Math.sin(endAngle);
          const largeArc = sliceAngle > Math.PI ? 1 : 0;

          const pathData = `M ${cx} ${cy} L ${x1} ${y1} A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2} Z`;
          const color = COLORS[i % COLORS.length];

          return (
            <g key={i}>
              <path d={pathData} fill={color} stroke="#ffffff" strokeWidth="2" className="hover:opacity-80 transition-opacity" />
              <title>{`${categories[i] || `Item ${i}`}: ${v}`}</title>
            </g>
          );
        })}
      </svg>
    );
  }

  // Column / Bar / Line / Scatter
  const plotWidth = width - padding * 2;
  const plotHeight = height - padding * 2;
  const stepX = plotWidth / Math.max(values.length, 1);

  return (
    <svg width={width} height={height} className="overflow-visible">
      {/* Grid Lines */}
      {[0, 0.25, 0.5, 0.75, 1].map((pct) => {
        const y = padding + plotHeight * (1 - pct);
        const valLabel = (minVal + range * pct).toFixed(0);
        return (
          <g key={pct}>
            <line x1={padding} y1={y} x2={width - padding} y2={y} stroke="#e2e8f0" strokeDasharray="3 3" />
            <text x={padding - 8} y={y + 3} textAnchor="end" className="text-[9px] fill-slate-400 font-mono">
              {valLabel}
            </text>
          </g>
        );
      })}

      {/* Axis Lines */}
      <line x1={padding} y1={padding} x2={padding} y2={height - padding} stroke="#94a3b8" />
      <line x1={padding} y1={height - padding} x2={width - padding} y2={height - padding} stroke="#94a3b8" />

      {/* Column Chart */}
      {chart_type === 'Column' &&
        values.map((v, i) => {
          const barW = Math.max(12, stepX * 0.6);
          const barH = (v / range) * plotHeight;
          const x = padding + i * stepX + (stepX - barW) / 2;
          const y = height - padding - barH;
          const color = COLORS[i % COLORS.length];

          return (
            <g key={i}>
              <rect x={x} y={y} width={barW} height={Math.max(1, barH)} fill={color} rx="2" className="hover:opacity-80" />
              <text x={x + barW / 2} y={height - padding + 14} textAnchor="middle" className="text-[9px] fill-slate-600 font-sans">
                {categories[i]?.slice(0, 8) || i}
              </text>
              <title>{`${categories[i]}: ${v}`}</title>
            </g>
          );
        })}

      {/* Line Chart */}
      {chart_type === 'Line' && (
        <g>
          <polyline
            fill="none"
            stroke="#2563eb"
            strokeWidth="2.5"
            points={values
              .map((v, i) => {
                const x = padding + i * stepX + stepX / 2;
                const y = height - padding - (v / range) * plotHeight;
                return `${x},${y}`;
              })
              .join(' ')}
          />
          {values.map((v, i) => {
            const x = padding + i * stepX + stepX / 2;
            const y = height - padding - (v / range) * plotHeight;
            return (
              <circle key={i} cx={x} cy={y} r="4" fill="#2563eb" stroke="#ffffff" strokeWidth="2" />
            );
          })}
        </g>
      )}

      {/* Scatter Plot */}
      {chart_type === 'Scatter' &&
        values.map((v, i) => {
          const x = padding + i * stepX + stepX / 2;
          const y = height - padding - (v / range) * plotHeight;
          return (
            <circle key={i} cx={x} cy={y} r="5" fill="#f59e0b" stroke="#ffffff" strokeWidth="1.5" />
          );
        })}
    </svg>
  );
};
