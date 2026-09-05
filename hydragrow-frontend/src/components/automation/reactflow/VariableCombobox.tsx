export interface VariableComboboxProps {
  /** Base id for the input; the generated `<datalist>` uses `${id}-vars`. */
  id: string;
  ariaLabel: string;
  /** Raw text currently shown — a literal ("7.2") or a variable name
   * ("ph_target_now"). The caller decides how to interpret it. */
  value: string;
  availableVariables: readonly string[];
  onChange: (raw: string) => void;
  placeholder?: string;
  className?: string;
}

/** A free-typing text input backed by a native `<datalist>` of in-scope
 * context variables. Deliberately dependency-free (no combobox library):
 * the user can type a literal value OR pick/type one of the suggested
 * variable names — both land in the same `onChange(raw: string)` callback,
 * matching the design's "Input/Select field that accepts variables actually
 * functions as a combobox" requirement. */
export function VariableCombobox({
  id,
  ariaLabel,
  value,
  availableVariables,
  onChange,
  placeholder,
  className,
}: VariableComboboxProps) {
  const listId = `${id}-vars`;
  return (
    <>
      <input
        id={id}
        aria-label={ariaLabel}
        list={listId}
        className={className ?? 'ui-input px-1 py-1 text-xs'}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
      />
      <datalist id={listId}>
        {availableVariables.map((v) => (
          <option key={v} value={v} />
        ))}
      </datalist>
    </>
  );
}
