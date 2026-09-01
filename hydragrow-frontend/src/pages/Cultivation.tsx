import { TabShell } from '../components/ui/TabShell';
import { CropSeasons } from './CropSeasons';
import RecipeBuilder from './RecipeBuilder';
import DosingHistory from './DosingHistory';

const Cultivation = () => (
  <TabShell
    title="Canh tác"
    subtitle="Mùa vụ, công thức dinh dưỡng và lịch sử châm — cùng vòng đời một vụ trồng."
    defaultTabId="seasons"
    tabs={[
      { id: 'seasons', label: 'Mùa vụ', content: <CropSeasons variant="embedded" /> },
      { id: 'recipes', label: 'Công thức', content: <RecipeBuilder variant="embedded" /> },
      { id: 'dosing-history', label: 'Lịch sử châm', content: <DosingHistory variant="embedded" /> },
    ]}
  />
);

export default Cultivation;
