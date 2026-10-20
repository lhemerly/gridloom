import React, { useState, useEffect, useRef } from 'react';
import { Check, X } from 'lucide-react';
import { CellRenderData } from '../types';
import { coordToA1 } from '../utils/coords';

interface FormulaBarProps {
  selectedRow: number;
  selectedCol: number;
  activeCellData?: CellRenderData;
  onCommit: (rawInput: string) => void;
}

const COMMON_FUNCTIONS = [
  'SUM',
  'SUMIF',
  'SUMIFS',
  'AVERAGE',
  'COUNT',
  'COUNTA',
  'MIN',
  'MAX',
  'IF',
  'IFS',
  'XLOOKUP',
  'VLOOKUP',
  'INDEX',
  'MATCH',
  'FILTER',
  'SORT',
  'UNIQUE',
  'SEQUENCE',
  'SQLVALUE',
  'SQLTABLE',
  'CONCAT',
  'TEXTJOIN',
  'DATE',
  'TODAY',
  'PMT',
  'NPV',
];

export const FormulaBar: React.FC<FormulaBarProps> = ({
  selectedRow,
  selectedCol,
  activeCellData,
  onCommit,
}) => {
  const coordA1 = coordToA1(selectedRow, selectedCol);
  const [inputValue, setInputValue] = useState('');
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [filterSuggestions, setFilterSuggestions] = useState<string[]>([]);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (activeCellData) {
      if (activeCellData.formula) {
        setInputValue(activeCellData.formula);
      } else if (activeCellData.raw_value !== null && activeCellData.raw_value !== undefined) {
        setInputValue(String(activeCellData.raw_value));
      } else {
        setInputValue(activeCellData.display_text || '');
      }
    } else {
      setInputValue('');
    }
  }, [activeCellData, selectedRow, selectedCol]);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setInputValue(val);

    if (val.startsWith('=')) {
      const match = val.match(/=([A-Z_]*)$/i);
      if (match) {
        const query = match[1].toUpperCase();
        const filtered = COMMON_FUNCTIONS.filter((fn) => fn.startsWith(query));
        setFilterSuggestions(filtered);
        setShowSuggestions(filtered.length > 0);
        return;
      }
    }
    setShowSuggestions(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      setShowSuggestions(false);
      onCommit(inputValue);
    } else if (e.key === 'Escape') {
      setShowSuggestions(false);
      if (activeCellData) {
        setInputValue(activeCellData.formula || String(activeCellData.raw_value ?? ''));
      }
    }
  };

  const selectSuggestion = (fnName: string) => {
    setInputValue(`=${fnName}(`);
    setShowSuggestions(false);
    if (inputRef.current) {
      inputRef.current.focus();
    }
  };

  return (
    <div className="relative flex items-center border-b border-slate-200 bg-white px-3 py-1 gap-2 text-xs select-none">
      {/* Active Coord Display */}
      <div className="w-16 text-center font-mono font-semibold text-slate-800 bg-slate-50 border border-slate-200 py-1 rounded px-1.5 truncate">
        {coordA1}
      </div>

      {/* Badges */}
      <div className="flex items-center gap-1">
        {activeCellData?.is_staged && (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-amber-100 text-amber-800 border border-amber-300">
            STAGED
          </span>
        )}
        {activeCellData?.is_spill && (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-blue-100 text-blue-800 border border-blue-300">
            SPILL
          </span>
        )}
        {activeCellData?.is_error && (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-bold bg-rose-100 text-rose-800 border border-rose-300">
            ERROR
          </span>
        )}
        {activeCellData?.table_name && (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-emerald-50 text-emerald-700 border border-emerald-200">
            {activeCellData.table_name}
          </span>
        )}
      </div>

      {/* Action icons */}
      <div className="flex items-center gap-0.5 text-slate-400">
        <button
          onClick={() => {
            if (activeCellData) {
              setInputValue(activeCellData.formula || String(activeCellData.raw_value ?? ''));
            }
          }}
          className="p-1 hover:text-slate-600 rounded hover:bg-slate-100"
          title="Cancel"
        >
          <X className="w-3.5 h-3.5" />
        </button>
        <button
          onClick={() => onCommit(inputValue)}
          className="p-1 hover:text-emerald-600 rounded hover:bg-slate-100"
          title="Commit (Enter)"
        >
          <Check className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Function indicator fx */}
      <div className="font-serif italic font-bold text-slate-400 text-sm px-1">
        fx
      </div>

      {/* Formula Input */}
      <div className="relative flex-1">
        <input
          ref={inputRef}
          type="text"
          value={inputValue}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          placeholder="Enter value or formula (e.g. =SUM(A1:A10) or =SQLVALUE(&quot;LocalDB&quot;, &quot;SELECT sum(amount) FROM invoices&quot;))"
          className="w-full px-2 py-1 text-xs font-mono text-slate-800 bg-transparent border-0 focus:outline-none focus:ring-0"
        />

        {/* Autocomplete Dropdown */}
        {showSuggestions && filterSuggestions.length > 0 && (
          <div className="absolute left-0 top-full mt-1 w-64 max-h-48 overflow-y-auto bg-white border border-slate-200 rounded shadow-lg z-50 py-1">
            {filterSuggestions.map((fn) => (
              <button
                key={fn}
                onClick={() => selectSuggestion(fn)}
                className="w-full text-left px-3 py-1 text-xs hover:bg-emerald-50 hover:text-emerald-700 font-mono flex items-center justify-between"
              >
                <span className="font-bold">{fn}</span>
                <span className="text-[10px] text-slate-400">function</span>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
