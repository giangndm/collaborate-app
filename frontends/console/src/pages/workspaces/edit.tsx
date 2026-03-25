import { Edit, useForm } from "@refinedev/antd";
import { Form, Input, Select } from "antd";

export const WorkspaceEdit = () => {
    const { formProps, saveButtonProps, queryResult } = useForm({
        action: "edit",
        resource: "workspaces",
    });

    const handleOnFinish = (values: any) => {
        const payload = {
            ...values,
            guest_join_enabled: values.guest_join_enabled ?? false,
            token_ttl_seconds: values.token_ttl_seconds ?? 3600,
        };
        if (formProps.onFinish) {
            formProps.onFinish(payload);
        }
    };

    return (
        <Edit saveButtonProps={saveButtonProps} isLoading={queryResult?.isLoading}>
            <Form {...formProps} onFinish={handleOnFinish} layout="vertical">
                <Form.Item
                    label="Name"
                    name={["name"]}
                >
                    <Input />
                </Form.Item>
                <Form.Item
                    label="Status"
                    name={["status"]}
                >
                    <Select
                        options={[
                            { value: "active", label: "Active" },
                            { value: "suspended", label: "Suspended" },
                        ]}
                    />
                </Form.Item>
            </Form>
        </Edit>
    );
};
