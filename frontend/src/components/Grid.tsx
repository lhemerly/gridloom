import { displayValue } from '../utils/values';
import React, { useState, useRef, useEffect, useCallback } from 'react';
import { ViewportResponse, CellRenderData } from '../types';
import { colIndexToName, coordToA1 } from '../utils/coords';

interface GridProps {
  sheetName: string;
  viewport: ViewportResponse | null;
  selectedRow: number;
  selectedCol: number;
  onSelectCell: (row: number, col: number) => void;
  onCommitCell: (row: number, col: number, value: string) => void;
  onViewportChange: (startRow: number, startCol: number, endRow: number, endCol: number) => void;
}

const CELL_WIDTH = 110;
const CELL_HEIGHT = 26;
const HEADER_HEIGHT = 26;
const ROW_HEADER_WIDTH = 48;

export const Grid: React.FC<GridProps> = ({
  sheetName,
  viewport,
  selectedRow,
  selectedCol,
  onSelectCell,
  onCommitCell,
  onViewportChange,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [scrollLeft, setScrollLeft] = useState(0);
  const [containerSize, setContainerSize] = useState({ width: 1000, height: 600 });
  const [isEditing, setIsEditing] = useState(false);
  const [editValue, setEditValue] = useState('');
  const editInputRef = useRef<HTMLInputElement>(null);

  // Total grid boundaries
  const totalRows = Math.max(viewport?.total_rows ?? 200, 200);
  const totalCols = Math.max(viewport?.total_cols ?? 50, 50);

  // ResizeObserver to track container dimensions
  useEffect(() => {
    if (!containerRef.current) return;
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerSize({
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        });
      }
    });
    ro.observe(containerRef.current);
    return () => ro.disconnect();
  }, []);

  // Compute visible row and col indices
  const startRow = Math.max(0, Math.floor(scrollTop / CELL_HEIGHT) - 5);
  const visibleRowCount = Math.ceil(containerSize.height / CELL_HEIGHT) + 10;
  const endRow = Math.min(totalRows - 1, startRow + visibleRowCount);

  const startCol = Math.max(0, Math.floor(scrollLeft / CELL_WIDTH) - 2);
  const visibleColCount = Math.ceil(containerSize.width / CELL_WIDTH) + 5;
  const endCol = Math.min(totalCols - 1, startCol + visibleColCount);

  // Notify parent of viewport changes
  useEffect(() => {
    onViewportChange(startRow, startCol, endRow, endCol);
  }, [startRow, startCol, endRow, endCol, onViewportChange]);

  const handleScroll = (e: React.UIEvent<HTMLDivElement>) => {
    setScrollTop(e.currentTarget.scrollTop);
    setScrollLeft(e.currentTarget.scrollLeft);
  };

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (isEditing) {
        if (e.key === 'Enter') {
          e.preventDefault();
          onCommitCell(selectedRow, selectedCol, editValue);
          setIsEditing(false);
          onSelectCell(Math.min(totalRows - 1, selectedRow + 1), selectedCol);
        } else if (e.key === 'Escape') {
          setIsEditing(false);
        } else if (e.key === 'Tab') {
          e.preventDefault();
          onCommitCell(selectedRow, selectedCol, editValue);
          setIsEditing(false);
          onSelectCell(selectedRow, Math.min(totalCols - 1, selectedCol + 1));
        }
        return;
      }

      let newRow = selectedRow;
      let newCol = selectedCol;

      switch (e.key) {
        case 'ArrowUp':
          newRow = Math.max(0, selectedRow - 1);
          break;
        case 'ArrowDown':
          newRow = Math.min(totalRows - 1, selectedRow + 1);
          break;
        case 'ArrowLeft':
          newCol = Math.max(0, selectedCol - 1);
          break;
        case 'ArrowRight':
          newCol = Math.min(totalCols - 1, selectedCol + 1);
          break;
        case 'Tab':
          e.preventDefault();
          newCol = e.shiftKey ? Math.max(0, selectedCol - 1) : Math.min(totalCols - 1, selectedCol + 1);
          break;
        case 'Enter':
          e.preventDefault();
          if (e.shiftKey) {
            newRow = Math.max(0, selectedRow - 1);
          } else {
            // Start editing
            startInlineEditing();
            return;
          }
          break;
        default:
          if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
            // Typing directly into cell
            setEditValue(e.key);
            setIsEditing(true);
            return;
          }
          return;
      }

      if (newRow !== selectedRow || newCol !== selectedCol) {
        e.preventDefault();
        onSelectCell(newRow, newCol);
      }
    },
    [isEditing, editValue, selectedRow, selectedCol, totalRows, totalCols, onSelectCell, onCommitCell]
  );

  const startInlineEditing = () => {
    const key = coordToA1(selectedRow, selectedCol);
    const cell = viewport?.cells[key];
    const initial = cell?.formula || (cell?.raw_value !== null && cell?.raw_value !== undefined ? displayValue(cell.raw_value) : cell?.display_text || '');
    setEditValue(initial);
    setIsEditing(true);
  };

  useEffect(() => {
    if (isEditing && editInputRef.current) {
      editInputRef.current.focus();
    }
  }, [isEditing]);

  const renderedRows: number[] = [];
  for (let r = startRow; r <= endRow; r++) {
    renderedRows.push(r);
  }

  const renderedCols: number[] = [];
  for (let c = startCol; c <= endCol; c++) {
    renderedCols.push(c);
  }

  return (
    <div
      ref={containerRef}
      onScroll={handleScroll}
      onKeyDown={handleKeyDown}
      tabIndex={0}
      className="relative flex-1 overflow-auto outline-none bg-white select-none font-sans"
    >
      {/* Scrollable Spacer canvas */}
      <div
        style={{
          width: totalCols * CELL_WIDTH + ROW_HEADER_WIDTH,
          height: totalRows * CELL_HEIGHT + HEADER_HEIGHT,
          position: 'relative',
        }}
      >
        {/* Top-Left Corner Header */}
        <div
          style={{
            position: 'sticky',
            top: 0,
            left: 0,
            width: ROW_HEADER_WIDTH,
            height: HEADER_HEIGHT,
            zIndex: 40,
          }}
          className="bg-slate-100 border-r border-b border-slate-300 flex items-center justify-center text-[10px] text-slate-400 font-mono"
        >
          {sheetName}
        </div>

        {/* Sticky Column Headers (A, B, C...) */}
        {renderedCols.map((colIdx) => (
          <div
            key={`col-${colIdx}`}
            style={{
              position: 'sticky',
              top: 0,
              left: colIdx * CELL_WIDTH + ROW_HEADER_WIDTH,
              width: CELL_WIDTH,
              height: HEADER_HEIGHT,
              transform: `translateX(${colIdx * CELL_WIDTH + ROW_HEADER_WIDTH}px)`,
              positionAnchor: 'sticky',
              zIndex: 30,
            }}
            className={`absolute top-0 border-r border-b border-slate-300 flex items-center justify-center text-xs font-semibold select-none ${
              colIdx === selectedCol
                ? 'bg-emerald-100 text-emerald-900 font-bold'
                : 'bg-slate-100 text-slate-700'
            }`}
          >
            {colIndexToName(colIdx)}
          </div>
        ))}

        {/* Sticky Row Headers (1, 2, 3...) */}
        {renderedRows.map((rowIdx) => (
          <div
            key={`row-${rowIdx}`}
            style={{
              position: 'sticky',
              left: 0,
              top: rowIdx * CELL_HEIGHT + HEADER_HEIGHT,
              width: ROW_HEADER_WIDTH,
              height: CELL_HEIGHT,
              transform: `translateY(${rowIdx * CELL_HEIGHT + HEADER_HEIGHT}px)`,
              zIndex: 30,
            }}
            className={`absolute left-0 border-r border-b border-slate-300 flex items-center justify-center text-xs font-mono select-none ${
              rowIdx === selectedRow
                ? 'bg-emerald-100 text-emerald-900 font-bold'
                : 'bg-slate-100 text-slate-600'
            }`}
          >
            {rowIdx + 1}
          </div>
        ))}

        {/* Virtualized Cell Grid */}
        {renderedRows.map((rowIdx) =>
          renderedCols.map((colIdx) => {
            const coordKey = coordToA1(rowIdx, colIdx);
            const cell = viewport?.cells[coordKey];
            const isSelected = rowIdx === selectedRow && colIdx === selectedCol;

            let bgClass = 'bg-white';
            if (cell?.is_staged) {
              bgClass = 'bg-amber-50 text-amber-950 font-medium';
            } else if (cell?.is_spill) {
              bgClass = 'bg-blue-50/60 text-blue-900';
            } else if (cell?.is_error) {
              bgClass = 'bg-rose-50 text-rose-700 font-mono';
            } else if (cell?.is_table_header) {
              bgClass = 'bg-slate-100 font-semibold text-slate-800 border-b-2 border-slate-400';
            }

            return (
              <div
                key={coordKey}
                onClick={() => onSelectCell(rowIdx, colIdx)}
                onDoubleClick={() => {
                  onSelectCell(rowIdx, colIdx);
                  startInlineEditing();
                }}
                style={{
                  position: 'absolute',
                  left: colIdx * CELL_WIDTH + ROW_HEADER_WIDTH,
                  top: rowIdx * CELL_HEIGHT + HEADER_HEIGHT,
                  width: CELL_WIDTH,
                  height: CELL_HEIGHT,
                  zIndex: isSelected ? 20 : 10,
                }}
                className={`border-r border-b border-slate-200 px-1.5 flex items-center text-xs truncate overflow-hidden cursor-cell select-none transition-colors ${bgClass} ${
                  isSelected ? 'ring-2 ring-blue-500 ring-inset bg-blue-50/20' : ''
                }`}
                title={cell?.is_error ? cell.display_text : undefined}
              >
                {/* Visual Indicator Dots */}
                {cell?.is_staged && (
                  <span
                    className="w-1.5 h-1.5 rounded-full bg-amber-500 mr-1 flex-shrink-0"
                    title="Staged edit pending review"
                  />
                )}
                {cell?.is_spill && (
                  <span
                    className="w-1.5 h-1.5 rounded-full bg-blue-500 mr-1 flex-shrink-0"
                    title="Dynamic Spill"
                  />
                )}
                {cell?.is_error && (
                  <span
                    className="w-1.5 h-1.5 rounded-full bg-rose-500 mr-1 flex-shrink-0"
                    title="Formula Error"
                  />
                )}

                {/* Text Content */}
                <span className="truncate flex-1">
                  {cell?.display_text ?? ''}
                </span>

                {/* Fill Handle for selected cell */}
                {isSelected && !isEditing && (
                  <div className="absolute right-0 bottom-0 w-2 h-2 bg-blue-600 cursor-crosshair" />
                )}
              </div>
            );
          })
        )}

        {/* Inline Cell Editor Overlay */}
        {isEditing && (
          <div
            style={{
              position: 'absolute',
              left: selectedCol * CELL_WIDTH + ROW_HEADER_WIDTH,
              top: selectedRow * CELL_HEIGHT + HEADER_HEIGHT,
              width: Math.max(CELL_WIDTH, 160),
              height: CELL_HEIGHT,
              zIndex: 50,
            }}
          >
            <input
              ref={editInputRef}
              type="text"
              value={editValue}
              onChange={(e) => setEditValue(e.target.value)}
              onBlur={() => {
                onCommitCell(selectedRow, selectedCol, editValue);
                setIsEditing(false);
              }}
              className="w-full h-full px-1.5 text-xs font-sans border-2 border-blue-600 bg-white shadow-md outline-none"
            />
          </div>
        )}
      </div>
    </div>
  );
};
