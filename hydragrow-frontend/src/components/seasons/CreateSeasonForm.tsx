import React, { useState } from 'react';
import { Sprout } from 'lucide-react';
import { InputGroup } from '../ui/InputGroup';

interface CreateSeasonFormProps {
  isLoading: boolean;
  onCreateSeason: (name: string, plantType: string, description: string) => Promise<any>;
}

export const CreateSeasonForm: React.FC<CreateSeasonFormProps> = ({ isLoading, onCreateSeason }) => {
  const [newName, setNewName] = useState('');
  const [newPlant, setNewPlant] = useState('');
  const [newDesc, setNewDesc] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newName.trim()) return;
    const success = await onCreateSeason(newName, newPlant, newDesc);
    if (success) {
      setNewName('');
      setNewPlant('');
      setNewDesc('');
    }
  };

  return (
    <div className="bg-white border border-emerald-100 rounded-xl overflow-hidden mb-6 shadow-sm">
      <form onSubmit={handleSubmit} className="p-5 md:p-6 flex flex-col gap-5">
        <h2 className="text-base font-semibold text-emerald-950 flex items-center gap-2 border-b border-emerald-100 pb-4">
          <Sprout size={20} className="text-emerald-500" />
          Tạo mùa vụ mới
        </h2>
        <div className="space-y-4">
          <InputGroup
            label="Tên mùa vụ *"
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          <InputGroup
            label="Giống cây trồng"
            type="text"
            value={newPlant}
            onChange={(e) => setNewPlant(e.target.value)}
          />
          <div className="flex flex-col gap-1">
            <label className="text-sm font-medium text-emerald-900">Ghi chú ban đầu</label>
            <textarea
              rows={2}
              placeholder="Nguồn hạt giống, mục tiêu TDS/pH mong muốn..."
              value={newDesc}
              onChange={(e) => setNewDesc(e.target.value)}
              className="w-full bg-white border border-emerald-200 text-emerald-950 text-sm rounded-lg px-3 py-2.5 outline-none focus:border-emerald-600 hover:border-emerald-300 resize-none transition-colors"
            />
          </div>
        </div>
        <button
          type="submit"
          disabled={isLoading || !newName.trim()}
          className="w-full py-2.5 bg-blue-600 hover:bg-blue-500 text-white rounded-lg font-medium text-sm transition-colors disabled:opacity-50"
        >
          {isLoading ? 'Đang tạo...' : 'Tạo mùa vụ mới'}
        </button>
      </form>
    </div>
  );
};
