import {
    Close,
    Content,
    type DialogProps,
    Overlay,
    Portal,
    Root,
    Title as DialogTitle,
} from '@radix-ui/react-dialog';
import { VisuallyHidden } from '@radix-ui/react-visually-hidden';
import { Command as CommandPrimitive } from 'cmdk';
import * as React from 'react';

const CommandDialog = ({ children, ...props }: DialogProps) => {
    return (
        <Root {...props}>
            <Portal>
                <Overlay className="fixed inset-0 z-50 bg-primary/80 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
                <Content className="fixed left-[50%] top-[50%] z-50 grid w-full max-w-3xl translate-x-[-50%] translate-y-[-50%] gap-4 bg-primary p-0 shadow-lg duration-200 outline outline-1 outline-primary rounded-2xl data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] sm:rounded-lg">
                    <VisuallyHidden asChild>
                        <DialogTitle>Command Menu</DialogTitle>
                    </VisuallyHidden>
                    <Command className="[&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:font-medium [&_[cmdk-group-heading]]:text-muted-foreground [&_[cmdk-group]:not([hidden])_~[cmdk-group]]:pt-0 [&_[cmdk-group]]:px-2 [&_[cmdk-input-wrapper]_svg]:h-5 [&_[cmdk-input-wrapper]_svg]:w-5 [&_[cmdk-input]]:h-12 [&_[cmdk-item]]:px-2 [&_[cmdk-item]]:py-3 [&_[cmdk-item]_svg]:h-5 [&_[cmdk-item]_svg]:w-5">
                        {children}
                    </Command>
                    <Close className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 disabled:pointer-events-none data-[state=open]:bg-accent data-[state=open]:text-muted-foreground">
                        <span className="sr-only">Close</span>
                    </Close>
                </Content>
            </Portal>
        </Root>
    );
};

// Command components
const Command = React.forwardRef<
    React.ComponentRef<typeof CommandPrimitive>,
    React.ComponentPropsWithoutRef<typeof CommandPrimitive>
>((props, ref) => (
    <CommandPrimitive
        ref={ref}
        className="raycast-cmdk flex h-full w-full flex-col overflow-hidden rounded-2xl bg-primary text-primary shadow-2xl border border-primary max-w-2xl mx-auto my-4"
        {...props}
    />
));

Command.displayName = CommandPrimitive.displayName;

const CommandInput = React.forwardRef<
    React.ComponentRef<typeof CommandPrimitive.Input>,
    React.ComponentPropsWithoutRef<typeof CommandPrimitive.Input>
>((props, ref) => (
    <div
        className="raycast-cmdk-input flex items-center border-b border-primary px-3 py-1 bg-secondary"
        // eslint-disable-next-line react/no-unknown-property
        cmdk-input-wrapper=""
    >
        <CommandPrimitive.Input
            ref={ref}
            className="flex h-10 w-full rounded-md bg-transparent text-base outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50 text-primary"
            {...props}
        />
    </div>
));

CommandInput.displayName = CommandPrimitive.Input.displayName;

const CommandList = React.forwardRef<
    React.ComponentRef<typeof CommandPrimitive.List>,
    React.ComponentPropsWithoutRef<typeof CommandPrimitive.List>
>((props, ref) => (
    <CommandPrimitive.List
        ref={ref}
        className="raycast-cmdk-list max-h-[350px] overflow-y-auto overflow-x-hidden py-1"
        {...props}
    />
));

CommandList.displayName = CommandPrimitive.List.displayName;

const CommandEmpty = React.forwardRef<
    React.ComponentRef<typeof CommandPrimitive.Empty>,
    React.ComponentPropsWithoutRef<typeof CommandPrimitive.Empty>
>((props, ref) => (
    <CommandPrimitive.Empty
        ref={ref}
        className="py-8 text-center text-base text-muted-foreground"
        {...props}
    />
));

CommandEmpty.displayName = CommandPrimitive.Empty.displayName;

const CommandGroup = React.forwardRef<
    React.ComponentRef<typeof CommandPrimitive.Group>,
    React.ComponentPropsWithoutRef<typeof CommandPrimitive.Group>
>((props, ref) => (
    <CommandPrimitive.Group
        ref={ref}
        className="overflow-hidden p-0 text-primary [&_[cmdk-group-heading]]:px-3 [&_[cmdk-group-heading]]:py-1.5 [&_[cmdk-group-heading]]:text-xs [&_[cmdk-group-heading]]:font-semibold [&_[cmdk-group-heading]]:text-muted-foreground"
        {...props}
    />
));

CommandGroup.displayName = CommandPrimitive.Group.displayName;

const CommandSeparator = React.forwardRef<
    React.ComponentRef<typeof CommandPrimitive.Separator>,
    React.ComponentPropsWithoutRef<typeof CommandPrimitive.Separator>
>((props, ref) => (
    <CommandPrimitive.Separator ref={ref} className="-mx-2 my-2 h-px bg-secondary" {...props} />
));

CommandSeparator.displayName = CommandPrimitive.Separator.displayName;

const CommandItem = React.forwardRef<
    React.ComponentRef<typeof CommandPrimitive.Item>,
    React.ComponentPropsWithoutRef<typeof CommandPrimitive.Item>
>((props, ref) => (
    <CommandPrimitive.Item
        ref={ref}
        className="raycast-cmdk-item relative flex w-full cursor-pointer gap-2 select-none items-center rounded-lg px-3 py-1.5 text-base outline-none transition-colors data-[disabled=true]:pointer-events-none data-[selected=true]:bg-tertiary data-[selected=true]:text-primary data-[disabled=true]:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-5 [&_svg]:shrink-0"
        {...props}
    />
));

CommandItem.displayName = CommandPrimitive.Item.displayName;

const CommandShortcut = (props: React.HTMLAttributes<HTMLSpanElement>) => {
    return <span className="ml-auto text-xs tracking-widest text-muted-foreground" {...props} />;
};

CommandShortcut.displayName = 'CommandShortcut';

export {
    Command,
    CommandDialog,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
    CommandSeparator,
    CommandShortcut,
};
