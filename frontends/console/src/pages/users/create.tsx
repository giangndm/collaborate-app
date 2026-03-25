import { Create, useForm } from "@refinedev/antd";
import { Form, Input, Select } from "antd";

export const UserCreate = () => {
    const { formProps, saveButtonProps } = useForm();

    return (
        <Create saveButtonProps={saveButtonProps}>
            <Form {...formProps} layout="vertical">
                <Form.Item
                    label="User ID"
                    name="id"
                    rules={[{ required: true }]}
                >
                    <Input placeholder="usr_123" />
                </Form.Item>
                <Form.Item
                    label="Email"
                    name="email"
                    rules={[{ required: true, type: "email" }]}
                >
                    <Input placeholder="user@example.com" />
                </Form.Item>
                <Form.Item
                    label="Display Name"
                    name="display_name"
                    rules={[{ required: true }]}
                >
                    <Input placeholder="John Doe" />
                </Form.Item>
                <Form.Item
                    label="Global Role"
                    name="global_role"
                    rules={[{ required: true }]}
                    initialValue="member"
                >
                    <Select>
                        <Select.Option value="member">Member</Select.Option>
                        <Select.Option value="super_admin">Super Admin</Select.Option>
                    </Select>
                </Form.Item>
                <Form.Item
                    label="Status"
                    name="status"
                    rules={[{ required: true }]}
                    initialValue="active"
                >
                    <Select>
                        <Select.Option value="active">Active</Select.Option>
                        <Select.Option value="suspended">Suspended</Select.Option>
                        <Select.Option value="disabled">Disabled</Select.Option>
                    </Select>
                </Form.Item>
            </Form>
        </Create>
    );
};
