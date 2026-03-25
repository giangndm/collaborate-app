import { Create, useForm } from "@refinedev/antd";
import { Form, Input } from "antd";

export const WorkspaceCreate = () => {
    const { formProps, saveButtonProps } = useForm({
        action: "create",
        resource: "workspaces",
    });

    const handleOnFinish = (values: any) => {
        // Auto-generate ID if not provided, assuming backend expects a string UUID
        if (!values.id) {
            values.id = crypto.randomUUID();
        }
        if (formProps.onFinish) {
            formProps.onFinish(values);
        }
    };

    return (
        <Create saveButtonProps={saveButtonProps}>
            <Form {...formProps} onFinish={handleOnFinish} layout="vertical">
                <Form.Item
                    label="Name"
                    name={["name"]}
                    rules={[{ required: true }]}
                >
                    <Input />
                </Form.Item>
                <Form.Item
                    label="Slug"
                    name={["slug"]}
                    rules={[{ required: true }]}
                >
                    <Input />
                </Form.Item>
            </Form>
        </Create>
    );
};
