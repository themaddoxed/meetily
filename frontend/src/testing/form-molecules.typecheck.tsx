import type { Control } from 'react-hook-form';
import { FormInputItem } from '@/components/molecules/form-components/form-input-item';
import { SwitchInput } from '@/components/molecules/form-components/form-input-switch';
import { FormSelectItem } from '@/components/molecules/form-components/form-select-item';

interface FixtureValues {
  name: string;
  enabled: boolean;
  choice: string;
}

export function FormMoleculesTypecheckFixture({
  control,
}: {
  control: Control<FixtureValues>;
}) {
  return (
    <>
      <FormInputItem control={control} name="name" type="text" />
      <SwitchInput control={control} name="enabled" />
      <FormSelectItem
        control={control}
        name="choice"
        placeholder="Choose"
        label="Choice"
        formStyle=""
        formLabelStyle=""
        formControlStyle=""
        formMessageStyle=""
        options={[{ label: 'One', value: 'one' }]}
        selectLabel="Choices"
        defaultValue="one"
      />

    </>
  );
}
