import {
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormDescription,
} from '@/components/ui/form';
import { Switch } from '@/components/ui/switch';
import { Control, FieldPath, FieldValues } from 'react-hook-form'; // Import Control type

type IInpuItemProps<TFieldValues extends FieldValues> = {
  name: FieldPath<TFieldValues>;
  placeholder?: string;
  control: Control<TFieldValues>; // Add control prop of type Control
  label?: string;
  value?: string | number;
  formStyle?: string;
  formLabelStyle?: string;
  formControlStyle?: string;
  formMessageStyle?: string;
  defaultValue?: string | number;
  disabled?: boolean;
  description?: string;
  isFormDescription?: boolean;
};

export const SwitchInput = <TFieldValues extends FieldValues,>({
  control,
  label,
  description,
  name,
  isFormDescription = true,
  formStyle,
}: IInpuItemProps<TFieldValues>) => {
  return (
    <FormField
      control={control}
      name={name}
      render={({ field }) => (
        <FormItem
          className={`flex flex-row items-center justify-between rounded-lg border p-4 ${formStyle}`}
        >
          <div className="space-y-0.5">
            <FormLabel className="text-base">{label}</FormLabel>
            {isFormDescription && (
              <FormDescription>{description}</FormDescription>
            )}
          </div>
          <FormControl>
            <Switch
              checked={field.value}
              onCheckedChange={field.onChange}
            />
          </FormControl>
        </FormItem>
      )}
    />
  );
};
