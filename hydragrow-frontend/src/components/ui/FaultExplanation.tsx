import { get_fault_guide } from '../../../gleam_core/build/dev/javascript/gleam_core/faults.mjs';

export const getFaultGuide = (code?: string) => {
  if (!code) return null;
  const guide = get_fault_guide(code);
  
  // Unwrap Gleam Option (Som [0] / None)
  if (guide && guide[0]) {
    return guide[0];
  }
  return null;
};
