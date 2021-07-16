import os

if __name__ == '__main__':
    stat =  os.waitstatus_to_exitcode(os.system('diff -r ../../testing-data/dummy-dat ../../testing-data/dummy-dat3'))
    if stat < 0:
        stat = -1*stat

    os.system('rm -r ../../testing-data/dummy-dat3/*')
    os._exit(stat)
