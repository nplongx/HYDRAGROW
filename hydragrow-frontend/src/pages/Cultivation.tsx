import { TabShell } from '../components/ui/TabShell';
import { useSearchParams } from 'react-router-dom';
import { parseTab, serializeTab } from '../lib/routeState';
import { CropSeasons } from './CropSeasons';
import RecipeBuilder from './RecipeBuilder';
import DosingHistory from './DosingHistory';

const Cultivation = () => {
  const [searchParams, setSearchParams] = useSearchParams();
  const activeTabId = parseTab(searchParams.toString(), ['seasons', 'recipes', 'dosing-history'], 'seasons');
  return (
    <TabShell
      title="Canh tác"
      subtitle="Mùa vụ, công thức dinh dưỡng và lịch sử châm — cùng vòng đời một vụ trồng."
      defaultTabId="seasons"
      activeTabId={activeTabId}
      onTabChange={(tabId) => setSearchParams(serializeTab(searchParams.toString(), tabId, 'seasons'), { replace: true })}
      tabs={[
        { id: 'seasons', label: 'Mùa vụ', content: <CropSeasons variant="embedded" /> },
        { id: 'recipes', label: 'Công thức', content: <RecipeBuilder variant="embedded" /> },
        { id: 'dosing-history', label: 'Lịch sử châm', content: <DosingHistory variant="embedded" /> },
      ]}
    />
  );
};

export default Cultivation;
