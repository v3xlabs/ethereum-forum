import * as Dialog from '@radix-ui/react-dialog';
import { FiMenu } from 'react-icons/fi';
import { LuX } from 'react-icons/lu';

import { Sidebar } from '../Sidebar';

export const SidebarHamburger = () => {
    return (
        <Dialog.Root>
            <Dialog.Trigger asChild>
                <button className="md:hidden button aspect-square size-8 flex items-center justify-center">
                    <FiMenu />
                </button>
            </Dialog.Trigger>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/50 z-40" />
                <Dialog.Content className="fixed inset-y-0 left-0 w-full max-w-xl bg-primary overflow-y-auto z-50">
                    <Sidebar />
                    <Dialog.Close className="absolute top-2 right-2 rounded-md p-1 hover:bg-secondary">
                        <LuX className="size-5" />
                    </Dialog.Close>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
};
