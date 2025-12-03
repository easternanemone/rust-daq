import { useState, useCallback } from 'react';
import { DragDropContext, DropResult } from 'react-beautiful-dnd';
import ActionPalette from './ActionPalette';
import Timeline from './Timeline';
import ParameterInspector from './ParameterInspector';
import ScriptPreview from './ScriptPreview';
import { ActionBlock, ActionTemplate, ACTION_TEMPLATES, ExperimentPlan } from '../types/experiment';
import { Save, FolderOpen, Play } from 'lucide-react';
import { save, open } from '@tauri-apps/api/dialog';
import { writeTextFile, readTextFile } from '@tauri-apps/api/fs';

function ExperimentSequencer() {
  const [actions, setActions] = useState<ActionBlock[]>([]);
  const [selectedBlock, setSelectedBlock] = useState<ActionBlock | null>(null);
  const [experimentName, setExperimentName] = useState('Untitled Experiment');

  const createActionFromTemplate = (template: ActionTemplate): ActionBlock => {
    return {
      id: `action-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      type: template.type,
      params: { ...template.defaultParams },
      children: template.canHaveChildren ? [] : undefined,
    };
  };

  const handleAddAction = (template: ActionTemplate) => {
    const newAction = createActionFromTemplate(template);
    setActions([...actions, newAction]);
    setSelectedBlock(newAction);
  };

  const findBlockById = (
    blocks: ActionBlock[],
    id: string
  ): ActionBlock | null => {
    for (const block of blocks) {
      if (block.id === id) return block;
      if (block.children) {
        const found = findBlockById(block.children, id);
        if (found) return found;
      }
    }
    return null;
  };

  const updateBlockInArray = (
    blocks: ActionBlock[],
    blockId: string,
    updater: (block: ActionBlock) => ActionBlock
  ): ActionBlock[] => {
    return blocks.map((block) => {
      if (block.id === blockId) {
        return updater(block);
      }
      if (block.children) {
        return {
          ...block,
          children: updateBlockInArray(block.children, blockId, updater),
        };
      }
      return block;
    });
  };

  const handleUpdateParams = (blockId: string, params: Record<string, any>) => {
    setActions((prev) =>
      updateBlockInArray(prev, blockId, (block) => ({
        ...block,
        params,
      }))
    );
    // Update selected block reference
    if (selectedBlock?.id === blockId) {
      setSelectedBlock((prev) => (prev ? { ...prev, params } : null));
    }
  };

  const deleteBlockAtPath = (blocks: ActionBlock[], path: number[]): ActionBlock[] => {
    if (path.length === 1) {
      return blocks.filter((_, index) => index !== path[0]);
    }

    const [first, ...rest] = path;
    return blocks.map((block, index) => {
      if (index === first && block.children) {
        return {
          ...block,
          children: deleteBlockAtPath(block.children, rest),
        };
      }
      return block;
    });
  };

  const handleDeleteBlock = (path: number[]) => {
    setActions((prev) => deleteBlockAtPath(prev, path));
    setSelectedBlock(null);
  };

  const moveBlock = (
    source: { droppableId: string; index: number },
    destination: { droppableId: string; index: number },
    draggableId: string
  ) => {
    // Clone the actions array
    let newActions = [...actions];

    // Handle drag from palette
    if (source.droppableId === 'palette') {
      const templateType = draggableId.replace('palette-', '');
      const template = ACTION_TEMPLATES.find((t) => t.type === templateType);
      if (!template) return;

      const newAction = createActionFromTemplate(template);

      if (destination.droppableId === 'timeline') {
        newActions.splice(destination.index, 0, newAction);
      } else {
        // Drop into a container block
        const targetBlock = findBlockById(newActions, destination.droppableId);
        if (targetBlock?.children) {
          targetBlock.children.splice(destination.index, 0, newAction);
        }
      }

      setActions(newActions);
      setSelectedBlock(newAction);
      return;
    }

    // Handle reordering within timeline or moving between containers
    const getBlockList = (id: string): ActionBlock[] => {
      if (id === 'timeline') return newActions;
      const block = findBlockById(newActions, id);
      return block?.children || [];
    };

    const sourceList = getBlockList(source.droppableId);
    const destList = getBlockList(destination.droppableId);

    const [movedBlock] = sourceList.splice(source.index, 1);

    if (source.droppableId === destination.droppableId) {
      sourceList.splice(destination.index, 0, movedBlock);
    } else {
      destList.splice(destination.index, 0, movedBlock);
    }

    setActions([...newActions]);
  };

  const handleDragEnd = (result: DropResult) => {
    const { source, destination, draggableId } = result;

    if (!destination) return;
    if (
      source.droppableId === destination.droppableId &&
      source.index === destination.index
    ) {
      return;
    }

    moveBlock(source, destination, draggableId);
  };

  const handleSavePlan = async () => {
    const plan: ExperimentPlan = {
      name: experimentName,
      description: '',
      created: new Date().toISOString(),
      modified: new Date().toISOString(),
      actions,
    };

    const filePath = await save({
      filters: [
        {
          name: 'Experiment Plan',
          extensions: ['json'],
        },
      ],
      defaultPath: `${experimentName.replace(/\s+/g, '_')}.json`,
    });

    if (filePath) {
      await writeTextFile(filePath, JSON.stringify(plan, null, 2));
    }
  };

  const handleLoadPlan = async () => {
    const filePath = await open({
      filters: [
        {
          name: 'Experiment Plan',
          extensions: ['json'],
        },
      ],
      multiple: false,
    });

    if (filePath && typeof filePath === 'string') {
      const content = await readTextFile(filePath);
      const plan: ExperimentPlan = JSON.parse(content);
      setExperimentName(plan.name);
      setActions(plan.actions);
      setSelectedBlock(null);
    }
  };

  const handleSelectBlock = useCallback((block: ActionBlock) => {
    setSelectedBlock(block);
  }, []);

  return (
    <DragDropContext onDragEnd={handleDragEnd}>
      <div className="h-full flex flex-col bg-slate-900">
        {/* Header */}
        <div className="px-6 py-3 bg-slate-800 border-b border-slate-700 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <input
              type="text"
              value={experimentName}
              onChange={(e) => setExperimentName(e.target.value)}
              className="px-3 py-1.5 bg-slate-700 border border-slate-600 rounded text-white font-semibold focus:outline-none focus:border-blue-500"
            />
            <div className="text-xs text-slate-400">
              {actions.length} action{actions.length !== 1 ? 's' : ''}
            </div>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleLoadPlan}
              className="px-3 py-1.5 bg-slate-700 hover:bg-slate-600 border border-slate-600 rounded text-white text-sm flex items-center gap-1.5 transition-colors"
            >
              <FolderOpen size={14} />
              Load
            </button>
            <button
              onClick={handleSavePlan}
              className="px-3 py-1.5 bg-slate-700 hover:bg-slate-600 border border-slate-600 rounded text-white text-sm flex items-center gap-1.5 transition-colors"
            >
              <Save size={14} />
              Save
            </button>
            <button
              disabled
              className="px-3 py-1.5 bg-green-700/50 border border-green-600/50 rounded text-white text-sm flex items-center gap-1.5 cursor-not-allowed opacity-50"
              title="Experiment execution not yet implemented"
            >
              <Play size={14} />
              Run
            </button>
          </div>
        </div>

        {/* Main Layout */}
        <div className="flex-1 flex overflow-hidden">
          {/* Left: Action Palette */}
          <div className="w-64 border-r border-slate-700">
            <ActionPalette onAddAction={handleAddAction} />
          </div>

          {/* Center: Timeline */}
          <div className="flex-1 border-r border-slate-700">
            <Timeline
              actions={actions}
              selectedBlock={selectedBlock}
              onSelectBlock={handleSelectBlock}
              onDeleteBlock={handleDeleteBlock}
            />
          </div>

          {/* Right: Parameter Inspector */}
          <div className="w-80">
            <ParameterInspector
              block={selectedBlock}
              onUpdateParams={handleUpdateParams}
            />
          </div>
        </div>

        {/* Bottom: Script Preview */}
        <div className="h-64 border-t border-slate-700">
          <ScriptPreview actions={actions} />
        </div>
      </div>
    </DragDropContext>
  );
}

export default ExperimentSequencer;
