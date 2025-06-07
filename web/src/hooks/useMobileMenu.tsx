import { ReactNode, createContext, useContext, useState } from 'react';

interface MobileMenuContextType {
    rightContent: ReactNode | null;
    setRightContent: (content: ReactNode | null) => void;
}

const MobileMenuContext = createContext<MobileMenuContextType | undefined>(undefined);

export const MobileMenuProvider = ({ children }: { children: ReactNode }) => {
    const [rightContent, setRightContent] = useState<ReactNode | null>(null);

    return (
        <MobileMenuContext.Provider value={{ rightContent, setRightContent }}>
            {children}
        </MobileMenuContext.Provider>
    );
};

export const useMobileMenu = () => {
    const context = useContext(MobileMenuContext);

    if (!context) {
        throw new Error('useMobileMenu must be used within a MobileMenuProvider');
    }

    return context;
}; 