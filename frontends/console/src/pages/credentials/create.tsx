import { Create, useForm } from "@refinedev/antd";
import { Form, Input } from "antd";
import { useParams } from "react-router-dom";

export const CredentialCreate = () => {
    const { workspaceId } = useParams();
    const { formProps, saveButtonProps } = useForm({
        action: "create",
        resource: "credentials",
        meta: { workspaceId },
        redirect: "list",
    });

    return (
        <Create saveButtonProps={saveButtonProps}>
            <Form {...formProps} layout="vertical">
                <Form.Item
                    label="Label (Description)"
                    name={["label"]}
                    rules={[{ required: true }]}
                >
                    <Input placeholder="e.g. Production API Key" />
                </Form.Item>
            </Form>
        </Create>
    );
};
