import { Create, useForm, useSelect } from "@refinedev/antd";
import { Form, Select } from "antd";
import { useParams } from "react-router-dom";

export const MemberCreate = () => {
    const { workspaceId } = useParams();
    const { formProps, saveButtonProps } = useForm({
        action: "create",
        resource: "members",
        meta: { workspaceId },
        redirect: "list",
    });

    const { selectProps } = useSelect({
        resource: "member-candidates",
        meta: {
            workspaceId,
        },
        optionLabel: "display_name", // We'll show display_name (email)
        optionValue: "id",
        onSearch: (value: string) => [
            {
                field: "query",
                operator: "contains",
                value,
            },
        ],
    });

    return (
        <Create saveButtonProps={saveButtonProps} title="Add Member">
            <Form {...formProps} layout="vertical">
                <Form.Item
                    label="User"
                    name="user_id"
                    rules={[{ required: true }]}
                >
                    <Select 
                        {...selectProps} 
                        showSearch 
                        placeholder="Search users..."
                        filterOption={false}
                    />
                </Form.Item>
                <Form.Item
                    label="Role"
                    name="role"
                    rules={[{ required: true }]}
                    initialValue="member"
                >
                    <Select>
                        <Select.Option value="owner">Owner</Select.Option>
                        <Select.Option value="member">Member</Select.Option>
                    </Select>
                </Form.Item>
            </Form>
        </Create>
    );
};
