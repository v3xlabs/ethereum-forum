import * as Dialog from '@radix-ui/react-dialog';
import classNames from 'classnames';
import { ReactNode } from 'react';
import { FiMenu } from 'react-icons/fi';
import { LuX } from 'react-icons/lu';

export const RightbarHamburger = ({
    children,
    triggerClassName = '',
}: {
    children: ReactNode;
    triggerClassName?: string;
}) => {
    return (
        <Dialog.Root>
            <Dialog.Trigger asChild>
                <button
                    className={classNames(
                        'md:hidden button aspect-square size-8 flex items-center justify-center',
                        triggerClassName
                    )}
                >
                    <FiMenu />
                </button>
            </Dialog.Trigger>
            <Dialog.Portal>
                <Dialog.Overlay className="fixed inset-0 bg-black/50 z-40 data-[state=open]:animate-overlayShow" />
                <Dialog.Content className="fixed inset-y-0 right-0 w-64 max-w-full bg-primary p-4 overflow-y-auto z-50 data-[state=open]:animate-contentShow">
                    {children}
                    <Dialog.Close className="absolute top-2 right-2 rounded-md p-1 hover:bg-secondary">
                        <LuX className="size-5" />
                    </Dialog.Close>
                </Dialog.Content>
            </Dialog.Portal>
        </Dialog.Root>
    );
};
