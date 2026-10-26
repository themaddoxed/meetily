import {useEffect, useRef} from 'react';

interface MessageToastProps {
    message: string;
    type: 'success' | 'error';
    show: boolean;
    setShow: (show: boolean) => void;
}

export function MessageToast({ message, type, show, setShow }: MessageToastProps) {
    const setShowRef = useRef(setShow);

    useEffect(() => {
        setShowRef.current = setShow;
    }, [setShow]);
    
    useEffect(() => {
        const timer = setTimeout(() => {
            setShowRef.current(false);
        }, 3000);
        
        return () => clearTimeout(timer);
    }, []);
    
    return (
        show && (
            <span className={`${type === 'success' ? 'text-success' : 'text-destructive'}`}>{message}</span>
        )
    );
}
