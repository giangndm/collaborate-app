import { Create, useForm } from "@refinedev/antd";
import { Form, Input, Select } from "antd";
import { useParams } from "react-router-dom";

export const MemberCreate = () => {
    const { workspaceId } = useParams();
    const { formProps, saveButtonProps } = useForm({
        action: "create",
        resource: "members",
        meta: { workspaceId },
        redirect: "list",
    });

    return (
        <Create saveButtonProps={saveButtonProps}>
            <Form {...formProps} layout="vertical">
                <Form.Item
                    label="User ID"
                    name={["user_id"]}
                    rules={[{ required: true }]}
                >
                    <Input />
                </Form.Item>
                <Form.Item
                    label="Role"
                    name={["role"]}
                    rules={[{ required: true }]}
                    initialValue="owner"
                >
                    <Select
                        options={[
                            { value: "owner", label: "Owner" },
                            { value: "member", label: "Member" },
                        ]}
                    />
                </Form.Item>
            </Form>
        </Create>
    );
};
